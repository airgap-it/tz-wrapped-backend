use actix_web::{Error, HttpResponse, error, web};
use diesel::{r2d2::ConnectionManager, prelude::*};
use r2d2::PooledConnection;
use uuid::Uuid;

use crate::DbPool;
use crate::db::models::{ operation_request::OperationRequest, contract::Contract, gatekeeper::Gatekeeper };
use crate::db::schema::{ gatekeepers, operation_requests, contracts };
use crate::api::models::{ operations::OperationBody, operations::NewOperation, operations::OperationResponse };


pub async fn post_operations(pool: web::Data<DbPool>, body: web::Json<OperationBody>) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("Failed to get a connection from the pool");

    let result = web::block(move || {
        let gatekeeper = body.find_and_validate_gatekeeper(&conn)?;

        let operation = NewOperation {
            requester: gatekeeper.id,
            destination: body.destination,
            target_address: body.target_address.clone(),
            amount: body.amount,
            kind: body.kind as i16,
            gk_signature: body.gk_signature.clone(),
            chain_id: body.chain_id.clone(),
            nonce: body.nonce
        };

        store_operation(&conn, &operation)
    })
    .await
    .map_err(|e| {
        eprintln!("{}", e);
        error::ErrorBadRequest(e)
    })?;

    Ok(HttpResponse::Ok().json(result))
}

fn store_operation(conn: &PooledConnection<ConnectionManager<PgConnection>>, operation: &NewOperation) -> Result<OperationResponse, diesel::result::Error> {
    let inserted_operation: OperationRequest = diesel::insert_into(operation_requests::dsl::operation_requests)
        .values(operation)
        .get_result(conn)?;
    
    let gatekeeper = get_gatekeeper(conn, inserted_operation.requester)?;
    let contract = get_contract(conn, inserted_operation.destination)?;

    let result = OperationResponse::from(inserted_operation, gatekeeper, contract);
    
    Ok(result)
}

fn get_gatekeeper(conn: &PooledConnection<ConnectionManager<PgConnection>>, id: Uuid) -> Result<Gatekeeper, diesel::result::Error> {
    let result: Gatekeeper = gatekeepers::dsl::gatekeepers.find(id).first(conn)?;

    Ok(result)
}

fn get_contract(conn: &PooledConnection<ConnectionManager<PgConnection>>, id: Uuid) -> Result<Contract, diesel::result::Error> {
    let result: Contract = contracts::dsl::contracts.find(id).first(conn)?;

    Ok(result)
}
