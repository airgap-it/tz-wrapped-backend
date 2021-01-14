use std::convert::TryInto;

use actix_web::{
    web,
    web::{Path, Query},
    HttpResponse,
};
use diesel::{r2d2::ConnectionManager, r2d2::PooledConnection, PgConnection};
use serde::Deserialize;
use uuid::Uuid;

use crate::api::models::{
    common::ListResponse, error::APIError, operation_approval::NewOperationApproval,
    operation_request::ApprovableOperationRequest, operation_request::OperationRequest,
    operation_request::OperationRequestKind,
};
use crate::db::models::{
    contract::Contract, operation_request::OperationRequest as DBOperationRequest, user::User,
};
use crate::settings;
use crate::tezos;
use crate::tezos::contract::multisig;
use crate::tezos::contract::{contract_call_for, multisig::Signature};
use crate::DbPool;

#[derive(Deserialize)]
pub struct Info {
    kind: OperationRequestKind,
    contract_id: Uuid,
    page: Option<i64>,
    limit: Option<i64>,
}

pub async fn get_operation_requests(
    pool: web::Data<DbPool>,
    query: Query<Info>,
) -> Result<HttpResponse, APIError> {
    let conn = pool.get()?;

    let page = query.page.unwrap_or(0);
    let limit = query.limit.unwrap_or(10);
    let kind = query.kind;
    let contract_id = query.contract_id;
    let result =
        web::block(move || load_operation_requests(&conn, page, limit, kind, contract_id)).await?;

    Ok(HttpResponse::Ok().json(result))
}

fn load_operation_requests(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    page: i64,
    limit: i64,
    kind: OperationRequestKind,
    contract_id: Uuid,
) -> Result<ListResponse<OperationRequest>, APIError> {
    let (operations_with_gatekeepers, total_pages) =
        DBOperationRequest::get_list(conn, kind, contract_id, page, limit)?;

    let results = operations_with_gatekeepers
        .into_iter()
        .map(|op| OperationRequest::from(op.0, op.1))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ListResponse {
        page,
        total_pages,
        results,
    })
}

async fn load_operation_and_contract(
    pool: &web::Data<DbPool>,
    operation_request_id: &Uuid,
) -> Result<(DBOperationRequest, Contract), APIError> {
    let conn = pool.get()?;
    let id = operation_request_id.clone();
    let result = web::block::<_, _, APIError>(move || {
        let operation_request = DBOperationRequest::get_by_id(&conn, &id)?;
        let contract = Contract::get_by_id(&conn, operation_request.contract_id)?;

        Ok((operation_request, contract))
    })
    .await?;

    Ok(result)
}

#[derive(Deserialize)]
pub struct PathInfo {
    id: Uuid,
}

pub async fn get_operation_request(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
) -> Result<HttpResponse, APIError> {
    let conn = pool.get()?;
    let id = path.id;

    let (operation_request, gatekeeper) = web::block::<_, _, APIError>(move || {
        let operation_request = DBOperationRequest::get_by_id(&conn, &id)?;
        let gatekeeper = User::get_by_id(&conn, operation_request.gatekeeper_id)?;

        Ok((operation_request, gatekeeper))
    })
    .await?;

    Ok(HttpResponse::Ok().json(OperationRequest::from(operation_request, gatekeeper)?))
}

pub async fn get_signable_message(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
    tezos_settings: web::Data<settings::Tezos>,
) -> Result<HttpResponse, APIError> {
    let id = path.id;
    let (operation, contract) = load_operation_and_contract(&pool, &id).await?;

    let multisig = multisig::get_multisig(
        contract.multisig_pkh.as_ref(),
        contract.kind.try_into()?,
        tezos_settings.node_url.as_ref(),
    );

    let message = tezos::contract::get_signable_message(
        &contract,
        operation.kind.try_into()?,
        operation.target_address.as_ref(),
        operation.amount,
        operation.nonce,
        operation.chain_id.as_ref(),
        &multisig,
    )
    .await?;

    Ok(HttpResponse::Ok().json(ApprovableOperationRequest {
        unsigned_operation_approval: NewOperationApproval {
            operation_request_id: id,
            signature: String::from(""),
        },
        signable_message: message,
    }))
}

pub async fn get_operation_request_parameters(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
    tezos_settings: web::Data<settings::Tezos>,
) -> Result<HttpResponse, APIError> {
    let conn = pool.get()?;
    let id = path.id;
    let (operation, contract, approvals) = web::block::<_, _, APIError>(move || {
        let operation_request = DBOperationRequest::get_by_id(&conn, &id)?;
        let contract = Contract::get_by_id(&conn, operation_request.contract_id)?;
        let approvals = operation_request.approvals(&conn)?;

        Ok((operation_request, contract, approvals))
    })
    .await?;

    let mut multisig = multisig::get_multisig(
        contract.multisig_pkh.as_ref(),
        contract.kind.try_into()?,
        tezos_settings.node_url.as_ref(),
    );
    let call = contract_call_for(
        &contract,
        operation.kind.try_into()?,
        operation.target_address.as_ref(),
        operation.amount,
    )?;
    let signatures = approvals
        .iter()
        .map(|(approval, user)| Signature {
            value: approval.signature.as_ref(),
            public_key: user.public_key.as_ref(),
        })
        .collect::<Vec<Signature>>();
    let parameters = multisig
        .parameters_for_call(call, operation.nonce, signatures, &contract.pkh)
        .await?;

    Ok(HttpResponse::Ok().json(parameters))
}
