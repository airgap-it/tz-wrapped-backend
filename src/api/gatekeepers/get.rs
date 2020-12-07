
use actix_web::{Error, HttpResponse, error, web::Query, web};
use diesel::{r2d2::ConnectionManager, r2d2::PooledConnection, prelude::*};
use serde::{ Deserialize };

use crate::{ DbPool };
use crate::db::models::{ gatekeeper::Gatekeeper };
use crate::db::schema::{ gatekeepers };
use crate::api::models::{ common::ListResponse, pagination::* };

#[derive(Deserialize)]
pub struct Info {
    page: Option<i64>,
    limit: Option<i64>
}

pub async fn get_gatekeepers(pool: web::Data<DbPool>, query: Query<Info>) -> Result<HttpResponse, Error> {
    let conn = pool.get().map_err(|e| {
        eprintln!("{}", e);
        error::ErrorInternalServerError(e)
    })?;

    let page = query.page.unwrap_or(0);
    let limit = query.limit.unwrap_or(10);

    let result = web::block(move || load_gatekeepers(&conn, page, limit))
        .await
        .map_err(|e| {
            eprintln!("{}", e);
            error::ErrorBadRequest(e)
        })?;
    
    Ok(HttpResponse::Ok().json(result))
}

fn load_gatekeepers(conn: &PooledConnection<ConnectionManager<PgConnection>>, page: i64, limit: i64) -> Result<ListResponse<Gatekeeper>, diesel::result::Error> {
    let gatekeepers_query = gatekeepers::dsl::gatekeepers
        .order_by(gatekeepers::dsl::created_at)
        .paginate(page)
        .per_page(limit);

    let (gatekeepers, total_pages) = gatekeepers_query.load_and_count_pages::<Gatekeeper>(&conn)?;
    
    Ok(ListResponse {
        page,
        total_pages,
        results: gatekeepers
    })
}
