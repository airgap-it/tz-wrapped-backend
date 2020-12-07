use actix_web::{HttpResponse, error, web::Query, web, Error};
use diesel::{r2d2::ConnectionManager, prelude::*};
use r2d2::PooledConnection;
use serde::{ Deserialize };

use crate::DbPool;
use crate::db::models::{ contract::Contract, gatekeeper::Gatekeeper, operation_request::OperationRequest };
use crate::db::schema::{ gatekeepers, operation_requests, contracts };
use crate::api::models::{ common::ListResponse, operations::OperationResponse, operations::OperationKind, pagination::* };

#[derive(Deserialize)]
pub struct Info {
    kind: OperationKind,
    page: Option<i64>,
    limit: Option<i64>
}

pub async fn get_operations(pool: web::Data<DbPool>, query: Query<Info>) -> Result<HttpResponse, Error> {
    let conn = pool.get().map_err(|e| {
        eprintln!("{}", e);
        error::ErrorInternalServerError(e)
    })?;
    
    let page = query.page.unwrap_or(0);
    let limit = query.limit.unwrap_or(10);
    let kind = query.kind;
    let result = web::block(move || load_operations(&conn, page, limit, kind))
        .await
        .map_err(|e| {
            eprintln!("{}", e);
            error::ErrorBadRequest(e)
        })?;
    
    Ok(HttpResponse::Ok().json(result))
}

fn load_operations(conn: &PooledConnection<ConnectionManager<PgConnection>>, page: i64, limit: i64, kind: OperationKind) -> Result<ListResponse<OperationResponse>, diesel::result::Error> {
    let operations_query = operation_requests::dsl::operation_requests
        .filter(operation_requests::dsl::kind.eq(kind as i16))
        .order_by(operation_requests::dsl::created_at)
        .inner_join(gatekeepers::table)
        .paginate(page)
        .per_page(limit);
    
    let (operations_with_gatekeepers, total_pages) = operations_query
        .load_and_count_pages::<(OperationRequest, Gatekeeper)>(&conn)?;
    
    let contracts = contracts::dsl::contracts
        .load::<Contract>(conn)?;

    // TODO: check of there is a better way to do this (fetch associated contracts)
    let results = operations_with_gatekeepers.into_iter().map(|op | {
        let contract = contracts.iter().find(|contract| { 
            contract.id.eq(&op.0.destination) 
        }).expect("Cannot find contract");
       OperationResponse::from(op.0, op.1, contract.clone()) 
    }).collect();

    Ok(ListResponse {
        page,
        total_pages,
        results
    })
}
