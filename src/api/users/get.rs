use std::convert::TryFrom;

use actix_web::{
    web,
    web::{Path, Query},
    HttpResponse,
};
use diesel::{prelude::*, r2d2::ConnectionManager, r2d2::PooledConnection};
use serde::Deserialize;
use uuid::Uuid;

use crate::api::models::{
    common::ListResponse,
    error::APIError,
    pagination::*,
    users::{UserKind, UserResponse, UserState},
};
use crate::db::models::user::User;
use crate::db::schema::users;
use crate::DbPool;

#[derive(Deserialize)]
pub struct Info {
    page: Option<i64>,
    limit: Option<i64>,
    kind: Option<UserKind>,
    contract: Option<Uuid>,
    state: Option<UserState>,
}

pub async fn get_users(
    pool: web::Data<DbPool>,
    query: Query<Info>,
) -> Result<HttpResponse, APIError> {
    let conn = pool.get()?;

    let page = query.page.unwrap_or(0);
    let limit = query.limit.unwrap_or(10);

    let result =
        web::block(move || load_users(&conn, page, limit, query.kind, query.contract, query.state))
            .await?;

    Ok(HttpResponse::Ok().json(result))
}

fn load_users(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    page: i64,
    limit: i64,
    kind: Option<UserKind>,
    contract: Option<Uuid>,
    state: Option<UserState>,
) -> Result<ListResponse<UserResponse>, APIError> {
    let mut users_query = users::dsl::users
        .filter(users::dsl::state.eq(state.unwrap_or(UserState::Active) as i16))
        .order_by(users::dsl::created_at)
        .into_boxed();

    if let Some(kind) = kind {
        users_query = users_query.filter(users::dsl::kind.eq(kind as i16));
    }

    if let Some(contract) = contract {
        users_query = users_query.filter(users::dsl::contract_id.eq(contract));
    }

    let paginated_query = users_query.paginate(page).per_page(limit);

    let (users, total_pages) = paginated_query.load_and_count_pages::<User>(&conn)?;

    let user_responses = users
        .into_iter()
        .map(|user| UserResponse::try_from(user))
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

    let user = web::block(move || User::get_by_id(&conn, id)).await?;

    Ok(HttpResponse::Ok().json(UserResponse::try_from(user)?))
}
