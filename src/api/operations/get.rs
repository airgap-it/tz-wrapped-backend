use std::convert::TryFrom;

use actix_web::{
    web,
    web::{Path, Query},
    HttpResponse,
};
use diesel::{prelude::*, r2d2::ConnectionManager};
use r2d2::PooledConnection;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    api::models::operations::ApprovableOperation,
    db::schema::{contracts, operation_requests, users},
};
use crate::{
    api::models::{approvals::PostOperationApprovalBody, operations::OperationState},
    tezos::contract::{contract_call_for, multisig::Signature},
    DbPool,
};
use crate::{
    api::models::{
        common::ListResponse, error::APIError, operations::OperationKind,
        operations::OperationRequestResponse, pagination::*,
    },
    settings,
};
use crate::{
    db::models::{contract::Contract, operation_request::OperationRequest, user::User},
    tezos::{self, contract::multisig::Multisig},
};

#[derive(Deserialize)]
pub struct Info {
    kind: OperationKind,
    contract_id: Uuid,
    page: Option<i64>,
    limit: Option<i64>,
}

pub async fn get_operations(
    pool: web::Data<DbPool>,
    query: Query<Info>,
) -> Result<HttpResponse, APIError> {
    let conn = pool.get()?;

    let page = query.page.unwrap_or(0);
    let limit = query.limit.unwrap_or(10);
    let kind = query.kind;
    let contract_id = query.contract_id;
    let result = web::block(move || load_operations(&conn, page, limit, kind, contract_id)).await?;

    Ok(HttpResponse::Ok().json(result))
}

fn load_operations(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    page: i64,
    limit: i64,
    kind: OperationKind,
    contract_id: Uuid,
) -> Result<ListResponse<OperationRequestResponse>, APIError> {
    let operations_query = operation_requests::dsl::operation_requests
        .filter(operation_requests::dsl::kind.eq(kind as i16))
        .filter(operation_requests::dsl::destination.eq(contract_id))
        .order_by(operation_requests::dsl::created_at)
        .inner_join(users::table)
        .paginate(page)
        .per_page(limit);

    let (operations_with_gatekeepers, total_pages) =
        operations_query.load_and_count_pages::<(OperationRequest, User)>(&conn)?;

    let results = operations_with_gatekeepers
        .into_iter()
        .map(|op| OperationRequestResponse::from(op.0, op.1))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ListResponse {
        page,
        total_pages,
        results,
    })
}

async fn load_operation_and_contract(
    pool: &web::Data<DbPool>,
    operation_id: &Uuid,
) -> Result<(OperationRequest, Contract), APIError> {
    let conn = pool.get()?;
    let id = operation_id.clone();
    let result = web::block::<_, _, APIError>(move || {
        let operation = OperationRequest::get_by_id(&conn, &id)?;
        let contract = Contract::get_by_id(&conn, operation.destination)?;

        Ok((operation, contract))
    })
    .await?;

    Ok(result)
}

#[derive(Deserialize)]
pub struct PathInfo {
    id: Uuid,
}

pub async fn get_operation(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
) -> Result<HttpResponse, APIError> {
    let conn = pool.get()?;
    let id = path.id;

    let (operation, gatekeeper) = web::block::<_, _, APIError>(move || {
        let operation = OperationRequest::get_by_id(&conn, &id)?;
        let gatekeeper = User::get_by_id(&conn, operation.requester)?;

        Ok((operation, gatekeeper))
    })
    .await?;

    Ok(HttpResponse::Ok().json(OperationRequestResponse::from(operation, gatekeeper)?))
}

pub async fn get_signable_message(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
    tezos_settings: web::Data<settings::Tezos>,
) -> Result<HttpResponse, APIError> {
    let id = path.id;
    let (operation, contract) = load_operation_and_contract(&pool, &id).await?;

    let multisig = Multisig::new(
        contract.multisig_pkh.as_ref(),
        tezos_settings.node_url.as_ref(),
    );

    let message = tezos::contract::get_signable_message(
        &contract,
        OperationKind::try_from(operation.kind)?,
        operation.target_address.as_ref(),
        operation.amount,
        operation.nonce,
        operation.chain_id.as_ref(),
        &multisig,
    )
    .await?;

    Ok(HttpResponse::Ok().json(ApprovableOperation {
        operation_approval: PostOperationApprovalBody {
            request: id,
            kh_signature: String::from(""),
        },
        signable_message: message,
    }))
}

pub async fn get_operation_parameters(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
    tezos_settings: web::Data<settings::Tezos>,
) -> Result<HttpResponse, APIError> {
    let conn = pool.get()?;
    let id = path.id;
    let (operation, contract, approvals) = web::block::<_, _, APIError>(move || {
        let operation = OperationRequest::get_by_id(&conn, &id)?;
        let contract = Contract::get_by_id(&conn, operation.destination)?;
        let approvals = operation.approvals(&conn)?;

        Ok((operation, contract, approvals))
    })
    .await?;

    let mut multisig = Multisig::new(
        contract.multisig_pkh.as_ref(),
        tezos_settings.node_url.as_ref(),
    );
    let call = contract_call_for(
        &contract,
        OperationKind::try_from(operation.kind)?,
        operation.target_address.as_ref(),
        operation.amount,
    )?;
    let signatures = approvals
        .iter()
        .map(|(approval, user)| Signature {
            value: approval.kh_signature.as_ref(),
            public_key: user.public_key.as_ref(),
        })
        .collect::<Vec<Signature>>();
    let parameters = multisig
        .parameters_for_call(call, operation.nonce, signatures, &contract.pkh)
        .await?;

    Ok(HttpResponse::Ok().json(parameters))
}
