use actix_web::{web, HttpResponse};

use crate::{
    api::models::{error::APIError, tezos_node::TezosNode},
    db::models::node_endpoint::NodeEndpoint,
    DbPool,
};

pub async fn selected_node(pool: web::Data<DbPool>) -> Result<HttpResponse, APIError> {
    let conn = pool.get()?;
    let node: TezosNode = web::block(move || NodeEndpoint::get_selected(&conn))
        .await?
        .into();
    Ok(HttpResponse::Ok().json(node))
}

pub async fn nodes(pool: web::Data<DbPool>) -> Result<HttpResponse, APIError> {
    let conn = pool.get()?;
    let result = web::block(move || NodeEndpoint::get_all(&conn)).await?;
    let response: Vec<TezosNode> = result
        .into_iter()
        .map(|node_endpoint| node_endpoint.into())
        .collect();

    Ok(HttpResponse::Ok().json(response))
}
