use std::{collections::HashSet, convert::TryInto, str::FromStr};

use actix_session::Session;
use actix_web::{web, HttpResponse};
use bigdecimal::BigDecimal;
use diesel::Connection;
use num_bigint::BigInt;

use crate::tezos::multisig;
use crate::DbPool;
use crate::{
    api::models::{
        error::APIError,
        operation_request::OperationRequest,
        operation_request::{NewOperationRequest, OperationRequestKind},
        user::{UserKind, UserState},
    },
    auth::get_current_user,
};
use crate::{
    db::models::{
        contract::Contract,
        operation_request::{
            NewOperationRequest as DBNewOperationRequest, OperationRequest as DBOperationRequest,
        },
        proposed_user::ProposedUser,
        user::{NewUser, User},
    },
    notifications::notify_new_operation_request,
};
use crate::{settings, tezos, tezos::coding::validate_edpk};

pub async fn operation_request(
    pool: web::Data<DbPool>,
    tezos_settings: web::Data<settings::Tezos>,
    new_operation_request: web::Json<NewOperationRequest>,
    server_settings: web::Data<settings::Server>,
    session: Session,
) -> Result<HttpResponse, APIError> {
    let current_user = get_current_user(&session, server_settings.inactivity_timeout_seconds)?;

    let conn = pool.get()?;
    let contract_id = new_operation_request.contract_id;

    current_user.require_roles(vec![UserKind::Gatekeeper], contract_id)?;

    let (contract, max_local_nonce) = web::block::<_, _, APIError>(move || {
        let contract = Contract::get(&conn, &contract_id)?;
        let max_nonce = DBOperationRequest::max_nonce(&conn, &contract.id).unwrap_or(-1);

        Ok((contract, max_nonce))
    })
    .await?;

    let mut multisig = multisig::get_multisig(
        contract.multisig_pkh.as_ref(),
        contract.kind.try_into()?,
        tezos_settings.node_url.as_ref(),
    );

    let nonce = std::cmp::max(multisig.nonce().await?, max_local_nonce + 1);
    let chain_id = tezos::chain_id(tezos_settings.node_url.as_ref()).await?;

    let amount = new_operation_request
        .amount
        .as_ref()
        .map(|amount| BigInt::from_str(amount.as_ref()))
        .map_or(Ok(None), |r| r.map(Some))?;

    let conn = pool.get()?;
    let (db_operation_request, gatekeeper, proposed_keyholders) =
        web::block::<_, _, APIError>(move || {
            conn.transaction(|| {
                let new_operation_request = new_operation_request.into_inner();
                let gatekeeper = User::get_active(
                    &conn,
                    &current_user.address,
                    UserKind::Gatekeeper,
                    contract_id,
                )?;

                let operation = DBNewOperationRequest {
                    gatekeeper_id: gatekeeper.id,
                    contract_id: new_operation_request.contract_id,
                    target_address: new_operation_request.target_address.clone(),
                    amount: amount.map(|amount| BigDecimal::new(amount, 0)),
                    threshold: new_operation_request.threshold,
                    kind: new_operation_request.kind.into(),
                    chain_id,
                    nonce,
                };

                operation.validate()?;

                let operation_request = DBOperationRequest::insert(&conn, &operation)?;
                let mut proposed_keyholder_users: Option<Vec<User>> = None;
                if new_operation_request.kind == OperationRequestKind::UpdateKeyholders {
                    if let Some(proposed_keyholders) = new_operation_request.proposed_keyholders {
                        let current_keyholders = User::get_all(
                            &conn,
                            Some(UserKind::Keyholder),
                            Some(new_operation_request.contract_id),
                            None,
                            None,
                            None,
                        )?;
                        let current_keyholders_set = current_keyholders
                            .iter()
                            .map(|user| &user.public_key)
                            .collect::<HashSet<_>>();

                        let proposed_keyholders_set =
                            proposed_keyholders.iter().collect::<HashSet<_>>();
                        let mut keyholders_to_add: Vec<NewUser> = Vec::new();
                        for public_key in
                            proposed_keyholders_set.difference(&current_keyholders_set)
                        {
                            validate_edpk(public_key)?;
                            keyholders_to_add.push(NewUser {
                                public_key: (**public_key).clone(),
                                address: tezos::edpk_to_tz1(public_key)?,
                                contract_id: new_operation_request.contract_id,
                                kind: UserKind::Keyholder.into(),
                                display_name: "".into(),
                                email: None,
                                state: UserState::Inactive.into(),
                            });
                        }
                        if !keyholders_to_add.is_empty() {
                            User::insert(&conn, keyholders_to_add)?;
                        }

                        let mut keyholders = User::get_all_matching_any(
                            &conn,
                            new_operation_request.contract_id,
                            UserKind::Keyholder.into(),
                            &proposed_keyholders,
                        )?;

                        keyholders.sort_unstable_by(|first, second| {
                            let first_position = proposed_keyholders
                                .iter()
                                .position(|public_key| public_key == &first.public_key)
                                .unwrap();
                            let second_position = proposed_keyholders
                                .iter()
                                .position(|public_key| public_key == &second.public_key)
                                .unwrap();
                            first_position.cmp(&second_position)
                        });

                        ProposedUser::insert(&conn, &operation_request, &keyholders)?;

                        proposed_keyholder_users = Some(keyholders);
                    }
                }

                Ok((operation_request, gatekeeper, proposed_keyholder_users))
            })
        })
        .await?;

    let signable_message = multisig
        .signable_message(
            &contract,
            &db_operation_request,
            proposed_keyholders.clone(),
        )
        .await?;

    let operation_request = OperationRequest::from(
        db_operation_request,
        gatekeeper,
        vec![],
        proposed_keyholders,
    )?;
    let operation_request_id = operation_request.id;

    let conn = pool.get()?;
    let _ = web::block::<_, _, APIError>(move || {
        let operation_request = DBOperationRequest::get(&conn, &operation_request_id)?;
        let contract = Contract::get(&conn, &operation_request.contract_id)?;
        let keyholders = User::get_all_active(&conn, contract.id, UserKind::Keyholder)?;
        let gatekeeper = User::get(&conn, operation_request.gatekeeper_id)?;
        let signable_message = signable_message.try_into()?;
        let _ = notify_new_operation_request(
            &gatekeeper,
            &keyholders,
            &operation_request,
            &signable_message,
            &contract,
        );

        Ok(())
    })
    .await;

    Ok(HttpResponse::Ok().json(operation_request))
}
