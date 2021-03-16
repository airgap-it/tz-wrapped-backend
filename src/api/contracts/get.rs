use std::convert::{TryFrom, TryInto};

use actix_web::{web, web::Path, web::Query, HttpResponse};
use diesel::{r2d2::ConnectionManager, r2d2::PooledConnection, PgConnection};
use serde::Deserialize;
use uuid::Uuid;

use crate::settings;
use crate::tezos::multisig;
use crate::DbPool;
use crate::{
    api::models::{common::ListResponse, contract::Contract, error::APIError},
    db::models::contract::Contract as DBContract,
    db::models::operation_request::OperationRequest,
};

#[derive(Deserialize)]
pub struct Info {
    page: Option<i64>,
    limit: Option<i64>,
}

pub async fn contracts(
    pool: web::Data<DbPool>,
    query: Query<Info>,
    contract_settings: web::Data<Vec<settings::Contract>>,
    tezos_settings: web::Data<settings::Tezos>,
) -> Result<HttpResponse, APIError> {
    DBContract::sync_contracts(&pool, &contract_settings, &tezos_settings.node_url).await?;

    let conn = pool.get()?;

    let page = query.page.unwrap_or(0);
    let limit = query.limit.unwrap_or(100);

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

pub async fn contract(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
) -> Result<HttpResponse, APIError> {
    let conn = pool.get()?;
    let contract_id = path.id;

    let contract = web::block(move || DBContract::get(&conn, &contract_id)).await?;

    Ok(HttpResponse::Ok().json(Contract::try_from(contract)?))
}

pub async fn contract_nonce(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
    tezos_settings: web::Data<settings::Tezos>,
) -> Result<HttpResponse, APIError> {
    let contract_id = path.id;
    let multisig_nonce =
        multisig_nonce(&pool, contract_id, tezos_settings.node_url.as_ref()).await?;

    Ok(HttpResponse::Ok().json(multisig_nonce))
}

pub async fn next_usable_nonce(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
    tezos_settings: web::Data<settings::Tezos>,
) -> Result<HttpResponse, APIError> {
    let contract_id = path.id;
    let multisig_nonce =
        multisig_nonce(&pool, contract_id, tezos_settings.node_url.as_ref()).await?;

    let conn = pool.get()?;
    let max_local_nonce = web::block::<_, _, APIError>(move || {
        Ok(OperationRequest::max_nonce(&conn, &contract_id).unwrap_or(-1))
    })
    .await?;

    let nonce = std::cmp::max(multisig_nonce, max_local_nonce + 1);

    Ok(HttpResponse::Ok().json(nonce))
}

async fn multisig_nonce(
    pool: &web::Data<DbPool>,
    contract_id: Uuid,
    node_url: &str,
) -> Result<i64, APIError> {
    let conn = pool.get()?;
    let contract = web::block(move || DBContract::get(&conn, &contract_id)).await?;
    let mut multisig = multisig::get_multisig(
        contract.multisig_pkh.as_ref(),
        contract.kind.try_into()?,
        node_url,
    );

    Ok(multisig.nonce().await?)
}
