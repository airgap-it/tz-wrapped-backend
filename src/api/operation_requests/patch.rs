use std::convert::TryInto;

use actix_session::Session;
use actix_web::{
    web::{self, Path},
    HttpResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::tezos::coding::validate_operation_hash;
use crate::DbPool;
use crate::{
    api::models::{
        error::APIError,
        operation_request::{OperationRequest, OperationRequestState, PatchOperationRequest},
        user::UserKind,
    },
    auth::get_current_user,
};
use crate::{
    db::models::{operation_request::OperationRequest as DBOperationRequest, user::User},
    settings,
};

#[derive(Deserialize)]
pub struct PathInfo {
    id: Uuid,
}

pub async fn operation_request(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
    patch_operation_request: web::Json<PatchOperationRequest>,
    server_settings: web::Data<settings::Server>,
    session: Session,
) -> Result<HttpResponse, APIError> {
    let current_user = get_current_user(&session, server_settings.inactivity_timeout_seconds)?;

    let conn = pool.get()?;
    let operation_request_id = path.id;

    if let Some(operation_hash) = &patch_operation_request.operation_hash {
        validate_operation_hash(operation_hash).map_err(|_error| APIError::InvalidValue {
            description: format!(
                "Provided operation hash ({}) is not a valid",
                operation_hash
            ),
        })?;
    }

    let (updated_operation, gatekeeper, operation_approvals, proposed_keyholders) =
        web::block::<_, _, APIError>(move || {
            let (operation_request, operation_approvals, proposed_keyholders) =
                DBOperationRequest::get_with_operation_approvals(&conn, &operation_request_id)?;

            current_user.require_roles(
                vec![UserKind::Gatekeeper, UserKind::Keyholder],
                operation_request.contract_id,
            )?;

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
            DBOperationRequest::mark_injected(
                &conn,
                &operation_request_id,
                patch_operation_request.operation_hash.clone(),
            )?;

            let updated_operation = DBOperationRequest::get(&conn, &operation_request_id)?;
            let gatekeeper = User::get(&conn, operation_request.gatekeeper_id)?;

            Ok((
                updated_operation,
                gatekeeper,
                operation_approvals,
                proposed_keyholders,
            ))
        })
        .await?;

    Ok(HttpResponse::Ok().json(OperationRequest::from(
        updated_operation,
        gatekeeper,
        operation_approvals,
        proposed_keyholders,
    )?))
}
