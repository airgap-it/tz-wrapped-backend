use std::convert::TryInto;

use actix_session::Session;
use actix_web::{
    web,
    web::{Path, Query},
    HttpResponse,
};
use diesel::{r2d2::ConnectionManager, r2d2::PooledConnection, PgConnection};
use serde::Deserialize;
use uuid::Uuid;

use crate::tezos;
use crate::tezos::contract::multisig;
use crate::tezos::contract::{contract_call_for, multisig::Signature};
use crate::DbPool;
use crate::{
    api::models::user::UserKind,
    db::models::{
        contract::Contract, operation_request::OperationRequest as DBOperationRequest, user::User,
    },
};
use crate::{
    api::models::{
        common::{ListResponse, SignableMessageInfo},
        error::APIError,
        operation_request::OperationRequest,
        operation_request::{OperationRequestKind, OperationRequestState},
    },
    auth::get_current_user,
};
use crate::{auth::SessionUser, settings};

#[derive(Deserialize)]
pub struct Info {
    kind: OperationRequestKind,
    contract_id: Uuid,
    state: Option<OperationRequestState>,
    page: Option<i64>,
    limit: Option<i64>,
}

pub async fn operation_requests(
    pool: web::Data<DbPool>,
    query: Query<Info>,
    session: Session,
) -> Result<HttpResponse, APIError> {
    let current_user = get_current_user(&session)?;

    let conn = pool.get()?;

    let page = query.page.unwrap_or(0);
    let limit = query.limit.unwrap_or(10);
    let kind = query.kind;
    let contract_id = query.contract_id;

    current_user.require_roles(vec![UserKind::Gatekeeper, UserKind::Keyholder], contract_id)?;

    let state = query.state;
    let result =
        web::block(move || load_operation_requests(&conn, page, limit, kind, contract_id, state))
            .await?;

    Ok(HttpResponse::Ok().json(result))
}

fn load_operation_requests(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    page: i64,
    limit: i64,
    kind: OperationRequestKind,
    contract_id: Uuid,
    state: Option<OperationRequestState>,
) -> Result<ListResponse<OperationRequest>, APIError> {
    let (operations_with_gatekeepers, total_pages) =
        DBOperationRequest::get_list(conn, kind, contract_id, state, page, limit)?;

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
    current_user: SessionUser,
) -> Result<(DBOperationRequest, Contract), APIError> {
    let conn = pool.get()?;
    let id = operation_request_id.clone();
    let result = web::block::<_, _, APIError>(move || {
        let operation_request = DBOperationRequest::get(&conn, &id)?;

        current_user.require_roles(
            vec![UserKind::Gatekeeper, UserKind::Keyholder],
            operation_request.contract_id,
        )?;

        let contract = Contract::get(&conn, &operation_request.contract_id)?;

        Ok((operation_request, contract))
    })
    .await?;

    Ok(result)
}

#[derive(Deserialize)]
pub struct PathInfo {
    id: Uuid,
}

pub async fn operation_request(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
    session: Session,
) -> Result<HttpResponse, APIError> {
    let current_user = get_current_user(&session)?;

    let conn = pool.get()?;
    let id = path.id;

    let (operation_request, gatekeeper) = web::block::<_, _, APIError>(move || {
        let operation_request = DBOperationRequest::get(&conn, &id)?;

        current_user.require_roles(
            vec![UserKind::Gatekeeper, UserKind::Keyholder],
            operation_request.contract_id,
        )?;

        let gatekeeper = User::get(&conn, operation_request.gatekeeper_id)?;

        Ok((operation_request, gatekeeper))
    })
    .await?;

    Ok(HttpResponse::Ok().json(OperationRequest::from(operation_request, gatekeeper)?))
}

pub async fn signable_message(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
    tezos_settings: web::Data<settings::Tezos>,
    session: Session,
) -> Result<HttpResponse, APIError> {
    let current_user = get_current_user(&session)?;

    let id = path.id;
    let (operation_request, contract) =
        load_operation_and_contract(&pool, &id, current_user).await?;

    let multisig = multisig::get_multisig(
        contract.multisig_pkh.as_ref(),
        contract.kind.try_into()?,
        tezos_settings.node_url.as_ref(),
    );

    let signable_message = tezos::contract::get_signable_message(
        &contract,
        operation_request.kind.try_into()?,
        operation_request.target_address.as_ref(),
        operation_request.amount.as_bigint_and_exponent().0,
        operation_request.nonce,
        operation_request.chain_id.as_ref(),
        &multisig,
    )
    .await?;

    let signable_message_info: SignableMessageInfo = signable_message.try_into()?;

    Ok(HttpResponse::Ok().json(signable_message_info))
}

pub async fn operation_request_parameters(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
    tezos_settings: web::Data<settings::Tezos>,
    session: Session,
) -> Result<HttpResponse, APIError> {
    let current_user = get_current_user(&session)?;

    let conn = pool.get()?;
    let id = path.id;
    let (operation_request, contract, approvals) = web::block::<_, _, APIError>(move || {
        let operation_request = DBOperationRequest::get(&conn, &id)?;

        current_user.require_roles(
            vec![UserKind::Gatekeeper, UserKind::Keyholder],
            operation_request.contract_id,
        )?;

        let contract = Contract::get(&conn, &operation_request.contract_id)?;
        let approvals = operation_request.operation_approvals(&conn)?;

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
        operation_request.kind.try_into()?,
        operation_request.target_address.as_ref(),
        operation_request.amount.as_bigint_and_exponent().0,
    )?;
    let signatures = approvals
        .iter()
        .map(|(approval, user)| Signature {
            value: approval.signature.as_ref(),
            public_key: user.public_key.as_ref(),
        })
        .collect::<Vec<Signature>>();
    let parameters = multisig
        .parameters_for_call(call, operation_request.nonce, signatures, &contract.pkh)
        .await?;

    Ok(HttpResponse::Ok().json(parameters))
}
