use crate::api::models::tezos_node::SelectedTezosNode;
use crate::api::models::tezos_node::TezosNode;
use crate::db::models::node_endpoint::NodeEndpoint;
use crate::settings;
use crate::DbPool;
use crate::{api::models::error::APIError, api::models::user::UserKind, auth::get_current_user};
use actix_session::Session;
use actix_web::{web, HttpResponse};
use log::info;

pub async fn mark_selected(
    pool: web::Data<DbPool>,
    body: web::Json<SelectedTezosNode>,
    server_settings: web::Data<settings::Server>,
    session: Session,
) -> Result<HttpResponse, APIError> {
    let current_user = get_current_user(&session, server_settings.inactivity_timeout_seconds)?;
    current_user.require_one_of_roles(vec![UserKind::Admin])?;

    let selected_node = body.into_inner();
    let conn = pool.get()?;
    let selected = web::block::<_, _, APIError>(move || {
        NodeEndpoint::set_selected(&conn, selected_node.id)?;
        Ok(NodeEndpoint::get_selected(&conn)?)
    })
    .await?;

    info!("Tezos node changed to: {:?}", selected.url);

    let response: TezosNode = selected.into();

    Ok(HttpResponse::Ok().json(response))
}
