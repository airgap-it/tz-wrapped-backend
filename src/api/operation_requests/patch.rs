use std::convert::TryInto;

use actix_web::{
    web::{self, Path},
    HttpResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::api::models::{
    error::APIError,
    operation_request::{OperationRequest, OperationRequestState, PatchOperationRequestBody},
};
use crate::db::models::{operation_request::OperationRequest as DBOperationRequest, user::User};
use crate::tezos::coding::validate_operation_hash;
use crate::DbPool;

#[derive(Deserialize)]
pub struct PathInfo {
    id: Uuid,
}

pub async fn patch_operation(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
    body: web::Json<PatchOperationRequestBody>,
) -> Result<HttpResponse, APIError> {
    let conn = pool.get()?;
    let id = path.id;

    validate_operation_hash(&body.operation_hash).map_err(|_error| APIError::InvalidValue {
        description: format!(
            "Provided operation hash ({}) is not a valid operation hash",
            body.operation_hash
        ),
    })?;

    let (updated_operation, gatekeeper) = web::block::<_, _, APIError>(move || {
        let operation_request = DBOperationRequest::get_by_id(&conn, &id)?;
        let state: OperationRequestState = operation_request.state.try_into()?;
        if state != OperationRequestState::Approved {
            return Err(APIError::InvalidOperationState {
                description: format!(
                    "Expected '{}', found '{}'",
                    OperationRequestState::Approved,
                    state
                ),
            });
        }
        DBOperationRequest::mark_injected(&conn, &id, body.operation_hash.clone())?;

        let updated_operation = DBOperationRequest::get_by_id(&conn, &id)?;
        let gatekeeper = User::get_by_id(&conn, operation_request.gatekeeper_id)?;

        Ok((updated_operation, gatekeeper))
    })
    .await?;

    Ok(HttpResponse::Ok().json(OperationRequest::from(updated_operation, gatekeeper)?))
}
