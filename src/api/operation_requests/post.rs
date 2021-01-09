use actix_web::{web, HttpResponse};
use diesel::{
    r2d2::{ConnectionManager, PooledConnection},
    PgConnection,
};

use crate::settings;
use crate::tezos::contract::get_signable_message;
use crate::tezos::contract::multisig::Multisig;
use crate::DbPool;
use crate::{
    api::models::{
        error::APIError, operation_request::NewOperationRequest,
        operation_request::OperationRequest, user::UserKind,
    },
    crypto,
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

pub async fn post_operation(
    pool: web::Data<DbPool>,
    tezos_settings: web::Data<settings::Tezos>,
    body: web::Json<NewOperationRequest>,
) -> Result<HttpResponse, APIError> {
    let conn = pool.get()?;
    let contract_id = body.contract_id;
    let contract = web::block(move || Contract::get_by_id(&conn, contract_id)).await?;

    let multisig = Multisig::new(
        contract.multisig_pkh.as_ref(),
        tezos_settings.node_url.as_ref(),
    );

    let message = get_signable_message(
        &contract,
        body.kind,
        body.target_address.as_ref(),
        body.amount,
        body.nonce.into(),
        body.chain_id.as_ref(),
        &multisig,
    )
    .await?;

    let conn = pool.get()?;
    let result = web::block(move || {
        let body = body.into_inner();
        let gatekeeper = find_and_validate_gatekeeper(&conn, &body, message)?;

        let operation = DBNewOperationRequest {
            gatekeeper_id: gatekeeper.id,
            contract_id: body.contract_id,
            target_address: body.target_address,
            amount: body.amount,
            kind: body.kind.into(),
            signature: body.signature,
            chain_id: body.chain_id,
            nonce: body.nonce,
        };

        let result = store_operation(&conn, &operation);

        if result.is_ok() {
            let contract = Contract::get_by_id(&conn, body.contract_id);
            if let Ok(contract) = contract {
                let keyholders = User::get_active(&conn, contract.id, UserKind::Keyholder);
                if let Ok(keyholders) = keyholders {
                    let _notification_result =
                        notify_new_operation_request(keyholders, body.kind, contract);
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
    message: String,
) -> Result<User, APIError> {
    let potential_gatekeepers =
        User::get_active(conn, operation_request.contract_id, UserKind::Gatekeeper)?;

    let message_bytes = hex::decode(message).map_err(|_error| APIError::InvalidValue {
        description: String::from("expected valid hex value"),
    })?;

    let hashed = crypto::generic_hash(&message_bytes, 32).map_err(|_error| APIError::Internal {
        description: String::from("hash failure"),
    })?;

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
