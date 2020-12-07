
use actix_web::{Error, HttpResponse, error, web::Query, web};
use diesel::{r2d2::ConnectionManager, r2d2::PooledConnection, prelude::*};
use serde::{ Deserialize };

use crate::{ DbPool };
use crate::db::models::contract::Contract;
use crate::db::schema::contracts;
use crate::api::models::{ common::ListResponse, pagination::* };

#[derive(Deserialize)]
pub struct Info {
    page: Option<i64>,
    limit: Option<i64>
}

pub async fn get_contracts(pool: web::Data<DbPool>, query: Query<Info>) -> Result<HttpResponse, Error> {
    let conn = pool.get().map_err(|e| {
        eprintln!("{}", e);
        error::ErrorInternalServerError(e)
    })?;

    let page = query.page.unwrap_or(0);
    let limit = query.limit.unwrap_or(10);

    let result = web::block(move || load_contracts(&conn, page, limit))
        .await
        .map_err(|e| {
            eprintln!("{}", e);
            error::ErrorBadRequest(e)
        })?;
    
    Ok(HttpResponse::Ok().json(result))
}

fn load_contracts(conn: &PooledConnection<ConnectionManager<PgConnection>>, page: i64, limit: i64) -> Result<ListResponse<Contract>, diesel::result::Error> {
    let contracts_query = contracts::dsl::contracts
        .order_by(contracts::dsl::created_at)
        .paginate(page)
        .per_page(limit);

    let (contracts, total_pages) = contracts_query.load_and_count_pages::<Contract>(&conn)?;
    
    Ok(ListResponse {
        page,
        total_pages,
        results: contracts
    })
}