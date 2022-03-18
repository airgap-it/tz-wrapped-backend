use std::{collections::HashSet, convert::TryFrom, convert::TryInto, str::FromStr};

use actix_session::Session;
use actix_web::{web, HttpResponse};
use bigdecimal::BigDecimal;
use diesel::Connection;
use log::info;
use num_bigint::BigInt;

use crate::db::models::node_endpoint::NodeEndpoint;
use crate::tezos::multisig::{self, OperationRequestParams, SignableMessage};
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
    new_operation_request: web::Json<NewOperationRequest>,
    server_settings: web::Data<settings::Server>,
    session: Session,
) -> Result<HttpResponse, APIError> {
    let new_operation_request = new_operation_request.into_inner();
    let current_user = get_current_user(&session, server_settings.inactivity_timeout_seconds)?;

    let conn = pool.get()?;
    let contract_id = new_operation_request.contract_id;
    let required_user_kind = match new_operation_request.kind {
        OperationRequestKind::UpdateKeyholders => UserKind::Keyholder,
        _ => UserKind::Gatekeeper,
    };
    current_user.require_roles(vec![required_user_kind], contract_id)?;

    let operation_request_kind: i16 = new_operation_request.kind.into();
    let (contract, max_local_nonce) = web::block::<_, _, APIError>(move || {
        let (contract, capabilities) = Contract::get_with_capabilities(&conn, &contract_id)?;
        let capability = capabilities
            .iter()
            .find(|cap| cap.operation_request_kind == operation_request_kind);
        if capability.is_none() {
            let kind: OperationRequestKind = operation_request_kind.try_into().unwrap();
            return Err(APIError::InvalidOperationRequest {
                description: format!(
                    "The multisig contract does not support operation requests of kind {}",
                    kind
                ),
            });
        }
        let max_nonce = DBOperationRequest::max_nonce(&conn, &contract.id).unwrap_or(-1);

        Ok((contract, max_nonce))
    })
    .await?;

    info!(
        "User {} submits new operation request on contract {}:\n{:?}",
        current_user.address, contract.display_name, new_operation_request
    );

    let conn = pool.get()?;
    let node_url =
        web::block::<_, _, APIError>(move || Ok(NodeEndpoint::get_selected(&conn)?.url)).await?;

    let node_url = &node_url;
    let mut multisig = multisig::get_multisig(
        contract.multisig_pkh.as_ref(),
        contract.kind.try_into()?,
        node_url,
    );

    let nonce = std::cmp::max(multisig.nonce().await?, max_local_nonce + 1);
    let chain_id = tezos::chain_id(node_url).await?;

    let amount = new_operation_request
        .amount
        .as_ref()
        .map(|amount| BigInt::from_str(amount.as_ref()))
        .map_or(Ok(None), |r| r.map(Some))?;

    let conn = pool.get()?;
    let ledger_hash = new_operation_request.ledger_hash.clone();

    let (new_db_operation, gatekeeper, proposed_keyholders_public_keys, contract_id) =
        web::block::<_, _, APIError>(move || {
            conn.transaction(|| {
                let user = User::get_active(
                    &conn,
                    &current_user.address,
                    required_user_kind,
                    contract_id,
                )?;

                let operation = DBNewOperationRequest {
                    user_id: user.id,
                    contract_id: new_operation_request.contract_id,
                    target_address: new_operation_request.target_address.clone(),
                    amount: amount.map(|amount| BigDecimal::new(amount, 0)),
                    threshold: new_operation_request.threshold,
                    kind: new_operation_request.kind.into(),
                    chain_id,
                    nonce,
                };

                operation.validate()?;

                let mut proposed_keyholders_public_keys: Option<Vec<String>> = None;
                if new_operation_request.kind == OperationRequestKind::UpdateKeyholders {
                    if let Some(proposed_keyholders) =
                        new_operation_request.proposed_keyholders.clone()
                    {
                        let proposed_keyholders_set =
                            proposed_keyholders.into_iter().collect::<HashSet<_>>();
                        let mut proposed_keyholder_pks = vec![];
                        for public_key in proposed_keyholders_set {
                            validate_edpk(public_key.as_str())?;
                            proposed_keyholder_pks.push(public_key)
                        }
                        proposed_keyholders_public_keys = Some(proposed_keyholder_pks)
                    }
                }

                Ok((
                    operation,
                    user,
                    proposed_keyholders_public_keys,
                    new_operation_request.contract_id,
                ))
            })
        })
        .await?;

    let operation_request_params = OperationRequestParams::from(new_db_operation.clone());
    let signable_message = multisig
        .signable_message(
            &contract,
            &operation_request_params,
            proposed_keyholders_public_keys.clone(),
        )
        .await?;

    verify_hash(&signable_message, ledger_hash)?;

    let conn = pool.get()?;
    let (db_operation_request, proposed_keyholders) = web::block::<_, _, APIError>(move || {
        conn.transaction(|| {
            let operation_request = DBOperationRequest::insert(&conn, &new_db_operation)?;
            let operation_request_kind = OperationRequestKind::try_from(operation_request.kind)?;
            let mut proposed_keyholder_users: Option<Vec<User>> = None;

            if operation_request_kind == OperationRequestKind::UpdateKeyholders {
                if let Some(proposed_keyholders) = proposed_keyholders_public_keys {
                    let current_keyholders = User::get_all(
                        &conn,
                        Some(UserKind::Keyholder),
                        Some(contract_id),
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
                    for public_key in proposed_keyholders_set.difference(&current_keyholders_set) {
                        validate_edpk(public_key)?;
                        keyholders_to_add.push(NewUser {
                            public_key: (**public_key).clone(),
                            address: tezos::edpk_to_tz1(public_key)?,
                            contract_id: contract_id,
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
                        contract_id,
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

            Ok((operation_request, proposed_keyholder_users))
        })
    })
    .await?;

    info!(
        "Successfully created operation request: {:?}",
        db_operation_request
    );

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
        let user = User::get(&conn, operation_request.user_id)?;
        let signable_message = signable_message.try_into()?;
        let _ = notify_new_operation_request(
            &user,
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

fn verify_hash(
    signable_message: &SignableMessage,
    maybe_ledger_hash: Option<String>,
) -> Result<(), APIError> {
    if let Some(ledger_hash) = maybe_ledger_hash {
        let expected_ledger_hash = signable_message.ledger_blake2b_hash()?;
        info!(
            "Verifying provided ledger hash {} with:\nData: {}\nData type: {}\nExpected ledger hash: {}",
            ledger_hash, signable_message.michelson_data, signable_message.michelson_type, expected_ledger_hash
        );
        if signable_message.ledger_blake2b_hash()? != ledger_hash {
            return Err(APIError::InvalidOperationRequest {
                description: "Invalid ledger hash".to_string(),
            });
        }
    }
    Ok(())
}
