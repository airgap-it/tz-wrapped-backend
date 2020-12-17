use actix_web::{web, HttpResponse};
use diesel::{prelude::*, r2d2::ConnectionManager};
use r2d2::PooledConnection;

use crate::db::models::{
    contract::Contract,
    operation_request::{NewOperationRequest, OperationRequest},
    user::User,
};
use crate::db::schema::operation_requests;
use crate::settings;
use crate::tezos::contract::get_signable_message;
use crate::tezos::contract::multisig::Multisig;
use crate::DbPool;
use crate::{
    api::models::{
        error::APIError,
        operations::OperationRequestResponse,
        operations::PostOperationRequestBody,
        users::{UserKind, UserState},
    },
    crypto,
};

pub async fn post_operations(
    pool: web::Data<DbPool>,
    tezos_settings: web::Data<settings::Tezos>,
    body: web::Json<PostOperationRequestBody>,
) -> Result<HttpResponse, APIError> {
    let mut conn = pool.get()?;
    let contract_id = body.destination;
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

    conn = pool.get()?;
    let result = web::block(move || {
        let body = body.into_inner();
        let gatekeeper = find_and_validate_gatekeeper(&conn, &body, message)?;

        let operation = NewOperationRequest {
            requester: gatekeeper.id,
            destination: body.destination,
            target_address: body.target_address,
            amount: body.amount,
            kind: body.kind.into(),
            gk_signature: body.gk_signature,
            chain_id: body.chain_id,
            nonce: body.nonce,
        };

        store_operation(&conn, &operation)
    })
    .await?;

    Ok(HttpResponse::Ok().json(result))
}

fn store_operation(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    operation: &NewOperationRequest,
) -> Result<OperationRequestResponse, APIError> {
    let inserted_operation: OperationRequest =
        diesel::insert_into(operation_requests::dsl::operation_requests)
            .values(operation)
            .get_result(conn)?;

    let gatekeeper = User::get_by_id(conn, inserted_operation.requester)?;

    let result = OperationRequestResponse::from(inserted_operation, gatekeeper)?;

    Ok(result)
}

pub fn find_and_validate_gatekeeper(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    operation_request: &PostOperationRequestBody,
    message: String,
) -> Result<User, APIError> {
    use crate::db::schema::users::dsl::*;

    let potential_gatekeepers: Vec<User> = users
        .filter(contract_id.eq(operation_request.destination))
        .filter(kind.eq(UserKind::Gatekeeper as i16))
        .filter(state.eq(UserState::Active as i16))
        .order_by(created_at)
        .load(conn)?;

    let message_bytes = hex::decode(message).map_err(|_error| APIError::InvalidValue {
        description: String::from("expected valid hex value"),
    })?;

    let hashed = crypto::generic_hash(&message_bytes, 32).map_err(|_error| APIError::Internal {
        description: String::from("hash failure"),
    })?;

    let mut result: Result<User, APIError> = Err(APIError::InvalidSignature);
    for gk in potential_gatekeepers {
        let is_match = gk.verify_message(&hashed, &operation_request.gk_signature)?;
        if is_match {
            result = Ok(gk);
            break;
        }
    }

    result
}
