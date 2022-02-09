use std::convert::TryFrom;

use actix_session::Session;
use actix_web::{
    web,
    web::{Path, Query},
    HttpResponse,
};
use diesel::{r2d2::ConnectionManager, r2d2::PooledConnection, PgConnection};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    api::models::{
        common::ListResponse,
        error::APIError,
        user::{User, UserKind, UserState},
    },
    auth::get_current_user,
};
use crate::{
    db::models::contract::Contract, db::models::user::User as DBUser, db::sync_keyholders, settings,
};
use crate::{db::models::node_endpoint::NodeEndpoint, DbPool};

#[derive(Deserialize)]
pub struct Info {
    page: Option<i64>,
    limit: Option<i64>,
    kind: Option<UserKind>,
    contract_id: Uuid,
    state: Option<UserState>,
    address: Option<String>,
}

pub async fn users(
    pool: web::Data<DbPool>,
    query: Query<Info>,
    server_settings: web::Data<settings::Server>,
    session: Session,
) -> Result<HttpResponse, APIError> {
    let current_user = get_current_user(&session, server_settings.inactivity_timeout_seconds)?;
    let contract_id = query.contract_id;
    current_user.require_roles(vec![UserKind::Gatekeeper, UserKind::Keyholder], contract_id)?;

    let conn = pool.get()?;
    let (contract, node_url) = web::block::<_, _, APIError>(move || {
        Ok((
            Contract::get(&conn, &contract_id)?,
            NodeEndpoint::get_selected(&conn)?.url,
        ))
    })
    .await?;

    sync_keyholders(&pool, vec![contract], &node_url).await?;

    let conn = pool.get()?;

    let page = query.page.unwrap_or(0);
    let limit = query.limit.unwrap_or(100);

    let result = web::block(move || {
        load_users(
            &conn,
            page,
            limit,
            query.kind,
            Some(contract_id),
            query.state,
            query.address.as_ref(),
        )
    })
    .await?;

    Ok(HttpResponse::Ok().json(result))
}

fn load_users(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    page: i64,
    limit: i64,
    kind: Option<UserKind>,
    contract_id: Option<Uuid>,
    state: Option<UserState>,
    address: Option<&String>,
) -> Result<ListResponse<User>, APIError> {
    let (users, total_pages) =
        DBUser::get_list(conn, state, kind, contract_id, address, page, limit)?;
    let user_responses = users
        .into_iter()
        .map(|user| User::try_from(user))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ListResponse {
        page,
        total_pages,
        results: user_responses,
    })
}

#[derive(Deserialize)]
pub struct PathInfo {
    id: Uuid,
}

pub async fn user(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
    server_settings: web::Data<settings::Server>,
    session: Session,
) -> Result<HttpResponse, APIError> {
    let current_user = get_current_user(&session, server_settings.inactivity_timeout_seconds)?;

    let conn = pool.get()?;
    let id = path.id;
    let user = web::block(move || DBUser::get(&conn, id)).await?;

    current_user.require_roles(
        vec![UserKind::Gatekeeper, UserKind::Keyholder],
        user.contract_id,
    )?;

    Ok(HttpResponse::Ok().json(User::try_from(user)?))
}
