use std::{convert::TryInto, str::FromStr};

use actix_session::Session;
use actix_web::{web, HttpResponse};
use bigdecimal::BigDecimal;
use diesel::{
    r2d2::{ConnectionManager, PooledConnection},
    PgConnection,
};
use num_bigint::BigInt;

use crate::tezos::contract::{get_signable_message, multisig};
use crate::DbPool;
use crate::{
    api::models::{
        error::APIError, operation_request::NewOperationRequest,
        operation_request::OperationRequest, user::UserKind,
    },
    auth::get_current_user,
};
use crate::{
    db::models::{
        contract::Contract,
        operation_request::{
            NewOperationRequest as DBNewOperationRequest, OperationRequest as DBOperationRequest,
        },
        user::User,
    },
    notifications::notify_new_operation_request,
};
use crate::{settings, tezos};

pub async fn new_operation_request(
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

    let amount = BigInt::from_str(new_operation_request.amount.as_ref())?;

    let signable_message = get_signable_message(
        &contract,
        new_operation_request.kind,
        new_operation_request.target_address.as_ref(),
        amount.clone(),
        nonce,
        chain_id.as_ref(),
        &multisig,
    )
    .await?;

    let conn = pool.get()?;
    let result = web::block(move || {
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
            amount: BigDecimal::new(amount, 0),
            kind: new_operation_request.kind.into(),
            chain_id,
            nonce,
        };

        let result = store_operation(&conn, &operation);

        if result.is_ok() {
            let contract = Contract::get(&conn, &new_operation_request.contract_id);
            if let Ok(contract) = contract {
                let keyholders = User::get_all_active(&conn, contract.id, UserKind::Keyholder);
                if let Ok(keyholders) = keyholders {
                    if let Ok(signable_message) = signable_message.try_into() {
                        let _notification_result = notify_new_operation_request(
                            &gatekeeper,
                            &keyholders,
                            &new_operation_request,
                            &signable_message,
                            &contract,
                        );
                    }
                }
            }
        }

        result
    })
    .await?;

    Ok(HttpResponse::Ok().json(result))
}

fn store_operation(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    new_operation_request: &DBNewOperationRequest,
) -> Result<OperationRequest, APIError> {
    let inserted_operation_request = DBOperationRequest::insert(conn, new_operation_request)?;
    let gatekeeper = User::get(conn, inserted_operation_request.gatekeeper_id)?;
    let result = OperationRequest::from(inserted_operation_request, gatekeeper, vec![])?;

    Ok(result)
}
