use std::{convert::TryInto, str::FromStr};

use actix_web::{web, HttpResponse};
use bigdecimal::BigDecimal;
use diesel::{
    r2d2::{ConnectionManager, PooledConnection},
    PgConnection,
};
use multisig::SignableMessage;
use num_bigint::BigInt;

use crate::api::models::{
    error::APIError, operation_request::NewOperationRequest, operation_request::OperationRequest,
    user::UserKind,
};
use crate::settings;
use crate::tezos::contract::{get_signable_message, multisig};
use crate::DbPool;
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

pub async fn post_operation(
    pool: web::Data<DbPool>,
    tezos_settings: web::Data<settings::Tezos>,
    body: web::Json<NewOperationRequest>,
) -> Result<HttpResponse, APIError> {
    let conn = pool.get()?;
    let contract_id = body.contract_id;
    let contract = web::block(move || Contract::get_by_id(&conn, contract_id)).await?;

    let multisig = multisig::get_multisig(
        contract.multisig_pkh.as_ref(),
        contract.kind.try_into()?,
        tezos_settings.node_url.as_ref(),
    );

    let amount = BigInt::from_str(body.amount.as_ref())?;

    let signable_message = get_signable_message(
        &contract,
        body.kind,
        body.target_address.as_ref(),
        amount.clone(),
        body.nonce.into(),
        body.chain_id.as_ref(),
        &multisig,
    )
    .await?;

    let conn = pool.get()?;
    let result = web::block(move || {
        let body = body.into_inner();
        let gatekeeper = find_and_validate_gatekeeper(&conn, &body, &signable_message)?;

        let operation = DBNewOperationRequest {
            gatekeeper_id: gatekeeper.id,
            contract_id: body.contract_id,
            target_address: body.target_address.clone(),
            amount: BigDecimal::new(amount, 0),
            kind: body.kind.into(),
            signature: body.signature.clone(),
            chain_id: body.chain_id.clone(),
            nonce: body.nonce,
        };

        let result = store_operation(&conn, &operation);

        if result.is_ok() {
            let contract = Contract::get_by_id(&conn, body.contract_id);
            if let Ok(contract) = contract {
                let keyholders = User::get_active(&conn, contract.id, UserKind::Keyholder);
                if let Ok(keyholders) = keyholders {
                    if let Ok(signable_message) = signable_message.try_into() {
                        let _notification_result = notify_new_operation_request(
                            &gatekeeper,
                            &keyholders,
                            &body,
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
    let gatekeeper = User::get_by_id(conn, inserted_operation_request.gatekeeper_id)?;
    let result = OperationRequest::from(inserted_operation_request, gatekeeper)?;

    Ok(result)
}

pub fn find_and_validate_gatekeeper(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    operation_request: &NewOperationRequest,
    message: &SignableMessage,
) -> Result<User, APIError> {
    let potential_gatekeepers =
        User::get_active(conn, operation_request.contract_id, UserKind::Gatekeeper)?;

    let hashed = message.blake2b_hash()?;

    let mut result: Result<User, APIError> = Err(APIError::InvalidSignature);
    for gk in potential_gatekeepers {
        let is_match = gk.verify_message(&hashed, &operation_request.signature)?;
        if is_match {
            result = Ok(gk);
            break;
        }
    }

    result
}
