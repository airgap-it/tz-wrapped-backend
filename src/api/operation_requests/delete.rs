use std::convert::TryInto;

use crate::{
    api::models::{error::APIError, user::UserKind},
    auth::get_current_user,
    db::models::{node_endpoint::NodeEndpoint, operation_request::OperationRequest},
    settings,
    tezos::multisig,
    DbPool,
};
use actix_session::Session;
use actix_web::{
    http::StatusCode,
    web::{self, Path},
    HttpResponse,
};
use log::info;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct PathInfo {
    id: Uuid,
}

pub async fn operation_request(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
    session: Session,
    server_settings: web::Data<settings::Server>,
) -> Result<HttpResponse, APIError> {
    let current_user = get_current_user(&session, server_settings.inactivity_timeout_seconds)?;

    let operation_request_id = path.id;
    let conn = pool.get()?;
    let (operation_request, contract) =
        web::block(move || OperationRequest::get_with_contract(&conn, &operation_request_id))
            .await?;

    current_user.require_roles(
        vec![UserKind::Gatekeeper, UserKind::Keyholder],
        operation_request.contract_id,
    )?;

    let conn = pool.get()?;
    let node_url =
        web::block::<_, _, APIError>(move || Ok(NodeEndpoint::get_selected(&conn)?.url)).await?;

    let mut multisig = multisig::get_multisig(
        contract.multisig_pkh.as_ref(),
        contract.kind.try_into()?,
        &node_url,
    );
    let multisig_nonce = multisig.nonce().await?;

    if operation_request.nonce < multisig_nonce {
        let conn = pool.get()?;
        let operation_request_id = operation_request.id;
        web::block(move || OperationRequest::delete(&conn, &operation_request.id)).await?;
        info!("Delete operation request {:?}", operation_request_id);
        return Ok(HttpResponse::Ok().status(StatusCode::NO_CONTENT).finish());
    }

    let conn = pool.get()?;
    let operation_request_id = operation_request.id;
    web::block(move || operation_request.delete_and_fix_next_nonces(&conn)).await?;
    info!("Delete operation request {:?}", operation_request_id);

    return Ok(HttpResponse::Ok().status(StatusCode::NO_CONTENT).finish());
}
