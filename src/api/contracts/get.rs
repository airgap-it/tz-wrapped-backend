use std::convert::{TryFrom, TryInto};

use crate::db::models::node_endpoint::NodeEndpoint;
use crate::tezos::multisig::{self};
use crate::DbPool;
use crate::{
    api::models::{common::ListResponse, contract::Contract, error::APIError},
    db::models::contract::Contract as DBContract,
    db::models::operation_request::OperationRequest,
};
use crate::{settings, Conn};
use actix_web::{web, web::Path, web::Query, HttpResponse};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Info {
    page: Option<i64>,
    limit: Option<i64>,
}

pub async fn contracts(
    pool: web::Data<DbPool>,
    query: Query<Info>,
    contract_settings: web::Data<Vec<settings::Contract>>,
) -> Result<HttpResponse, APIError> {
    let conn = pool.get()?;
    let node_url =
        web::block::<_, _, APIError>(move || Ok(NodeEndpoint::get_selected(&conn)?.url)).await?;
    DBContract::sync_contracts(&pool, &contract_settings, &node_url).await?;

    let conn = pool.get()?;

    let page = query.page.unwrap_or(0);
    let limit = query.limit.unwrap_or(100);

    let result = web::block(move || load_contracts(&conn, page, limit)).await?;

    Ok(HttpResponse::Ok().json(result))
}

fn load_contracts(conn: &Conn, page: i64, limit: i64) -> Result<ListResponse<Contract>, APIError> {
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

    let contract =
        web::block(move || DBContract::get_with_capabilities(&conn, &contract_id)).await?;

    Ok(HttpResponse::Ok().json(Contract::try_from(contract)?))
}

pub async fn contract_nonce(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
) -> Result<HttpResponse, APIError> {
    let contract_id = path.id;
    let conn = pool.get()?;
    let node_url =
        web::block::<_, _, APIError>(move || Ok(NodeEndpoint::get_selected(&conn)?.url)).await?;
    let multisig_nonce = multisig_nonce(&pool, contract_id, &node_url).await?;

    Ok(HttpResponse::Ok().json(multisig_nonce))
}

pub async fn next_usable_nonce(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
) -> Result<HttpResponse, APIError> {
    let contract_id = path.id;
    let conn = pool.get()?;
    let node_url =
        web::block::<_, _, APIError>(move || Ok(NodeEndpoint::get_selected(&conn)?.url)).await?;
    let multisig_nonce = multisig_nonce(&pool, contract_id, &node_url).await?;

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
