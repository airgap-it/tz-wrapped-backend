use std::convert::TryFrom;

use actix_web::{web, web::Path, web::Query, HttpResponse};
use diesel::{r2d2::ConnectionManager, r2d2::PooledConnection, PgConnection};
use serde::Deserialize;
use uuid::Uuid;

use crate::api::models::{
    common::ListResponse,
    contract::{Contract, SignableOperationRequest},
    error::APIError,
    operation_request::{NewOperationRequest, OperationRequestKind},
};
use crate::db::models::{contract::Contract as DBContract, operation_request::OperationRequest};
use crate::settings;
use crate::tezos;
use crate::tezos::contract::multisig::Multisig;
use crate::DbPool;

#[derive(Deserialize)]
pub struct Info {
    page: Option<i64>,
    limit: Option<i64>,
}

pub async fn get_contracts(
    pool: web::Data<DbPool>,
    query: Query<Info>,
) -> Result<HttpResponse, APIError> {
    let conn = pool.get()?;

    let page = query.page.unwrap_or(0);
    let limit = query.limit.unwrap_or(10);

    let result = web::block(move || load_contracts(&conn, page, limit)).await?;

    Ok(HttpResponse::Ok().json(result))
}

fn load_contracts(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    page: i64,
    limit: i64,
) -> Result<ListResponse<Contract>, APIError> {
    let (contracts, total_pages) = DBContract::get_list(conn, page, limit)?;
    let contract_responses = contracts
        .into_iter()
        .map(|contract| Contract::try_from(contract))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ListResponse {
        page,
        total_pages,
        results: contract_responses,
    })
}

#[derive(Deserialize)]
pub struct PathInfo {
    id: Uuid,
}

pub async fn get_contract(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
) -> Result<HttpResponse, APIError> {
    let conn = pool.get()?;
    let id = path.id;

    let contract = web::block(move || DBContract::get_by_id(&conn, id)).await?;

    Ok(HttpResponse::Ok().json(Contract::try_from(contract)?))
}

pub async fn get_contract_nonce(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
    tezos_settings: web::Data<settings::Tezos>,
) -> Result<HttpResponse, APIError> {
    let conn = pool.get()?;
    let id = path.id;

    let (contract, max_nonce) = web::block::<_, _, APIError>(move || {
        let contract = DBContract::get_by_id(&conn, id)?;
        let max_nonce = OperationRequest::max_nonce(&conn, &contract.id)?;

        Ok((contract, max_nonce))
    })
    .await?;
    let mut multisig = Multisig::new(
        contract.multisig_pkh.as_ref(),
        tezos_settings.node_url.as_ref(),
    );

    let multisig_nonce = multisig.nonce().await?;

    Ok(HttpResponse::Ok().json(std::cmp::max(multisig_nonce, max_nonce + 1)))
}

#[derive(Deserialize)]
pub struct SignableInfo {
    target_address: Option<String>,
    amount: i64,
    kind: OperationRequestKind,
}

pub async fn get_signable_message(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
    query: Query<SignableInfo>,
    tezos_settings: web::Data<settings::Tezos>,
) -> Result<HttpResponse, APIError> {
    let conn = pool.get()?;
    let id = path.id;
    let (contract, max_nonce) = web::block::<_, _, APIError>(move || {
        let contract = DBContract::get_by_id(&conn, id)?;
        let max_nonce = OperationRequest::max_nonce(&conn, &contract.id).unwrap_or(0);

        Ok((contract, max_nonce))
    })
    .await?;
    let mut multisig = Multisig::new(
        contract.multisig_pkh.as_ref(),
        tezos_settings.node_url.as_ref(),
    );

    let nonce = std::cmp::max(multisig.nonce().await?, max_nonce + 1);
    let chain_id = multisig.chain_id().await?;

    let message = tezos::contract::get_signable_message(
        &contract,
        query.kind,
        query.target_address.as_ref(),
        query.amount,
        nonce,
        chain_id.as_ref(),
        &multisig,
    )
    .await?;

    let operation_request = NewOperationRequest {
        contract_id: contract.id,
        target_address: query.target_address.clone(),
        amount: query.amount,
        kind: query.kind,
        signature: "".to_owned(),
        chain_id,
        nonce: nonce,
    };

    let response = SignableOperationRequest {
        unsigned_operation_request: operation_request,
        signable_message: message,
    };

    Ok(HttpResponse::Ok().json(response))
}
