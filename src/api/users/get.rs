use std::convert::TryFrom;

use actix_web::{
    web,
    web::{Path, Query},
    HttpResponse,
};
use diesel::{r2d2::ConnectionManager, r2d2::PooledConnection, PgConnection};
use serde::Deserialize;
use uuid::Uuid;

use crate::api::models::{
    common::ListResponse,
    error::APIError,
    user::{User, UserKind, UserState},
};
use crate::db::models::user::User as DBUser;
use crate::DbPool;

#[derive(Deserialize)]
pub struct Info {
    page: Option<i64>,
    limit: Option<i64>,
    kind: Option<UserKind>,
    contract_id: Option<Uuid>,
    state: Option<UserState>,
    address: Option<String>,
}

pub async fn get_users(
    pool: web::Data<DbPool>,
    query: Query<Info>,
) -> Result<HttpResponse, APIError> {
    let conn = pool.get()?;

    let page = query.page.unwrap_or(0);
    let limit = query.limit.unwrap_or(10);

    let result = web::block(move || {
        load_users(
            &conn,
            page,
            limit,
            query.kind,
            query.contract_id,
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

pub async fn get_user(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
) -> Result<HttpResponse, APIError> {
    let conn = pool.get()?;

    let id = path.id;

    let user = web::block(move || DBUser::get_by_id(&conn, id)).await?;

    Ok(HttpResponse::Ok().json(User::try_from(user)?))
}
