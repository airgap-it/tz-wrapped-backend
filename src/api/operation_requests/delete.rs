use std::convert::TryInto;

use actix_session::Session;
use actix_web::{
    http::StatusCode,
    web::{self, Path},
    HttpResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    api::models::{error::APIError, operation_request::OperationRequestState, user::UserKind},
    auth::get_current_user,
    db::models::operation_request::OperationRequest,
    settings,
    tezos::contract::multisig,
    DbPool,
};

#[derive(Deserialize)]
pub struct PathInfo {
    id: Uuid,
}

pub async fn operation_request(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
    session: Session,
    tezos_settings: web::Data<settings::Tezos>,
) -> Result<HttpResponse, APIError> {
    let current_user = get_current_user(&session)?;

    let operation_request_id = path.id;
    let conn = pool.get()?;
    let (operation_request, contract) =
        web::block(move || OperationRequest::get_with_contract(&conn, &operation_request_id))
            .await?;

    current_user.require_roles(
        vec![UserKind::Gatekeeper, UserKind::Keyholder],
        operation_request.contract_id,
    )?;

    let operation_request_state: OperationRequestState = operation_request.state.try_into()?;
    if operation_request_state == OperationRequestState::Injected {
        return Err(APIError::InvalidOperationState {
            description: "cannot delete operation with injected state".into(),
        });
    }

    let mut multisig = multisig::get_multisig(
        contract.multisig_pkh.as_ref(),
        contract.kind.try_into()?,
        tezos_settings.node_url.as_ref(),
    );
    let multisig_nonce = multisig.nonce().await?;

    if operation_request.nonce < multisig_nonce {
        let conn = pool.get()?;
        web::block(move || OperationRequest::delete(&conn, &operation_request.id)).await?;
        return Ok(HttpResponse::Ok().status(StatusCode::NO_CONTENT).finish());
    }

    let conn = pool.get()?;
    web::block(move || operation_request.delete_and_fix_next_nonces(&conn)).await?;

    return Ok(HttpResponse::Ok().status(StatusCode::NO_CONTENT).finish());
}
