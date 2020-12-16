use actix_web::{
    web,
    web::{Path, Query},
    HttpResponse,
};
use diesel::{prelude::*, r2d2::ConnectionManager, r2d2::PooledConnection};
use serde::Deserialize;
use uuid::Uuid;

use crate::api::models::{
    approvals::OperationApprovalResponse, common::ListResponse, error::APIError, pagination::*,
};
use crate::db::models::{operation_approval::OperationApproval, user::User};
use crate::db::schema::{operation_approvals, users};
use crate::DbPool;

#[derive(Deserialize)]
pub struct Info {
    request_id: Uuid,
    page: Option<i64>,
    limit: Option<i64>,
}

pub async fn get_approvals(
    pool: web::Data<DbPool>,
    query: Query<Info>,
) -> Result<HttpResponse, APIError> {
    let conn = pool.get().map_err(|error| APIError::DBError {
        description: error.to_string(),
    })?;

    let page = query.page.unwrap_or(0);
    let limit = query.limit.unwrap_or(10);
    let request_id = query.request_id;

    let result = web::block(move || load_approvals(&conn, request_id, page, limit)).await?;

    Ok(HttpResponse::Ok().json(result))
}

fn load_approvals(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    request_id: Uuid,
    page: i64,
    limit: i64,
) -> Result<ListResponse<OperationApprovalResponse>, APIError> {
    let approvals_query = operation_approvals::dsl::operation_approvals
        .filter(operation_approvals::request.eq(request_id))
        .order_by(operation_approvals::dsl::created_at)
        .inner_join(users::table)
        .paginate(page)
        .per_page(limit);

    let (operations_with_keyholders, total_pages) =
        approvals_query.load_and_count_pages::<(OperationApproval, User)>(&conn)?;

    let results = operations_with_keyholders
        .into_iter()
        .map(|approval| OperationApprovalResponse::from(approval.0, approval.1))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ListResponse {
        page,
        total_pages,
        results,
    })
}

#[derive(Deserialize)]
pub struct PathInfo {
    id: Uuid,
}

pub async fn get_approval(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
) -> Result<HttpResponse, APIError> {
    let conn = pool.get()?;
    let id = path.id;

    let approval = web::block::<_, _, APIError>(move || {
        let approval = OperationApproval::get_by_id(&conn, id)?;
        let keyholder = User::get_by_id(&conn, approval.approver)?;

        Ok((approval, keyholder))
    })
    .await?;

    Ok(HttpResponse::Ok().json(OperationApprovalResponse::from(approval.0, approval.1)?))
}
