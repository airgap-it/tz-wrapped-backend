use actix_session::Session;
use actix_web::{
    web,
    web::{Path, Query},
    HttpResponse,
};
use diesel::{r2d2::ConnectionManager, r2d2::PooledConnection, PgConnection};
use serde::Deserialize;
use uuid::Uuid;

use crate::DbPool;
use crate::{
    api::models::user::UserKind,
    db::models::{
        operation_approval::OperationApproval as DBOperationApproval,
        operation_request::OperationRequest, user::User,
    },
};
use crate::{
    api::models::{common::ListResponse, error::APIError, operation_approval::OperationApproval},
    auth::get_current_user,
};

#[derive(Deserialize)]
pub struct Info {
    operation_request_id: Uuid,
    page: Option<i64>,
    limit: Option<i64>,
}

pub async fn operation_approvals(
    pool: web::Data<DbPool>,
    query: Query<Info>,
    session: Session,
) -> Result<HttpResponse, APIError> {
    let current_user = get_current_user(&session)?;

    let conn = pool.get()?;
    let operation_request_id = query.operation_request_id;
    let operation_request =
        web::block(move || OperationRequest::get(&conn, &operation_request_id)).await?;

    current_user.require_roles(
        vec![UserKind::Gatekeeper, UserKind::Keyholder],
        operation_request.contract_id,
    )?;

    let page = query.page.unwrap_or(0);
    let limit = query.limit.unwrap_or(10);

    let conn = pool.get()?;
    let result =
        web::block(move || load_approvals(&conn, operation_request_id, page, limit)).await?;

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

pub async fn operation_approval(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
    session: Session,
) -> Result<HttpResponse, APIError> {
    let current_user = get_current_user(&session)?;

    let conn = pool.get()?;
    let id = path.id;

    let approval = web::block::<_, _, APIError>(move || {
        let approval = DBOperationApproval::get(&conn, id)?;
        let operation_request = OperationRequest::get(&conn, &approval.operation_request_id)?;

        current_user.require_roles(
            vec![UserKind::Gatekeeper, UserKind::Keyholder],
            operation_request.contract_id,
        )?;

        let keyholder = User::get(&conn, approval.keyholder_id)?;

        Ok((approval, keyholder))
    })
    .await?;

    Ok(HttpResponse::Ok().json(OperationApproval::from(approval.0, approval.1)?))
}
