use actix_session::Session;
use actix_web::{http::StatusCode, HttpResponse};

use crate::api::models::error::APIError;

pub async fn sign_out(session: Session) -> Result<HttpResponse, APIError> {
    session.clear();
    Ok(HttpResponse::Ok().status(StatusCode::NO_CONTENT).finish())
}
