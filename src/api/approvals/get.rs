use actix_web::{Error, HttpResponse, web::Query, error, web};
use diesel::{r2d2::ConnectionManager, r2d2::PooledConnection, prelude::*};
use serde::{ Deserialize };
use uuid::Uuid;

use crate::DbPool;
use crate::db::models::{ keyholder::Keyholder, operation_approval::OperationApproval };
use crate::db::schema::{ operation_approvals, keyholders };
use crate::api::models::{ common::ListResponse, approvals::ApprovalResponse, pagination::* };

#[derive(Deserialize)]
pub struct Info {
    request_id: Uuid,
    page: Option<i64>,
    limit: Option<i64>
}

pub async fn get_approvals(pool: web::Data<DbPool>, query: Query<Info>) -> Result<HttpResponse, Error> {
    let conn = pool.get().map_err(|e| {
        eprintln!("{}", e);
        error::ErrorInternalServerError(e)
    })?;

    let page = query.page.unwrap_or(0);
    let limit = query.limit.unwrap_or(10);
    let request_id = query.request_id;

    let result = web::block(move || load_approvals(&conn, request_id, page, limit))
        .await
        .map_err(|e| {
            eprintln!("{}", e);
            error::ErrorBadRequest(e)
        })?;
    
    Ok(HttpResponse::Ok().json(result))
}

fn load_approvals(conn: &PooledConnection<ConnectionManager<PgConnection>>, request_id: Uuid, page: i64, limit: i64) -> Result<ListResponse<ApprovalResponse>, diesel::result::Error> {
    let approvals_query = operation_approvals::dsl::operation_approvals
        .filter(operation_approvals::request.eq(request_id))
        .order_by(operation_approvals::dsl::created_at)
        .inner_join(keyholders::table)
        .paginate(page)
        .per_page(limit);
    
    let (operations_with_gatekeepers, total_pages) = approvals_query.load_and_count_pages::<(OperationApproval, Keyholder)>(&conn)?;

    let results = operations_with_gatekeepers.into_iter().map(|approval | {
        ApprovalResponse::from(approval.0, approval.1)
    }).collect();
    
    Ok(ListResponse {
        page,
        total_pages,
        results
    })
}
