use actix_web::{
    web,
    web::{Path, Query},
    HttpResponse,
};
use diesel::{r2d2::ConnectionManager, r2d2::PooledConnection, PgConnection};
use serde::Deserialize;
use uuid::Uuid;

use crate::api::models::{
    common::ListResponse, error::APIError, operation_approval::OperationApproval,
};
use crate::db::models::{operation_approval::OperationApproval as DBOperationApproval, user::User};
use crate::DbPool;

#[derive(Deserialize)]
pub struct Info {
    operation_request_id: Uuid,
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
    let request_id = query.operation_request_id;

    let result = web::block(move || load_approvals(&conn, request_id, page, limit)).await?;

    Ok(HttpResponse::Ok().json(result))
}

fn load_approvals(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    operation_request_id: Uuid,
    page: i64,
    limit: i64,
) -> Result<ListResponse<OperationApproval>, APIError> {
    let (operations_with_keyholders, total_pages) =
        DBOperationApproval::get_list(conn, operation_request_id, page, limit)?;
    let results = operations_with_keyholders
        .into_iter()
        .map(|approval| OperationApproval::from(approval.0, approval.1))
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
        let approval = DBOperationApproval::get_by_id(&conn, id)?;
        let keyholder = User::get_by_id(&conn, approval.keyholder_id)?;

        Ok((approval, keyholder))
    })
    .await?;

    Ok(HttpResponse::Ok().json(OperationApproval::from(approval.0, approval.1)?))
}
