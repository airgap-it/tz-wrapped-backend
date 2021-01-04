use std::convert::TryInto;

use actix_web::{
    web::{self, Path},
    HttpResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    api::models::{
        error::APIError,
        operations::{OperationRequestResponse, OperationState, PatchOperationRequestBody},
    },
    db::models::{operation_request::OperationRequest, user::User},
    tezos::coding::validate_operation_hash,
    DbPool,
};

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
        let operation = OperationRequest::get_by_id(&conn, &id)?;
        let state: OperationState = operation.state.try_into()?;
        if state != OperationState::Approved {
            return Err(APIError::InvalidOperationState {
                description: format!("Expected '{}', found '{}'", OperationState::Approved, state),
            });
        }
        OperationRequest::mark_injected(&conn, &id, body.operation_hash.clone())?;

        let updated_operation = OperationRequest::get_by_id(&conn, &id)?;
        let gatekeeper = User::get_by_id(&conn, operation.requester)?;

        Ok((updated_operation, gatekeeper))
    })
    .await?;

    Ok(HttpResponse::Ok().json(OperationRequestResponse::from(
        updated_operation,
        gatekeeper,
    )?))
}
