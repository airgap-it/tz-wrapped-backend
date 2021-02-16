use actix_session::Session;
use actix_web::{
    http::StatusCode,
    web::{self, Query},
    HttpResponse,
};
use serde::Deserialize;

use crate::{api::models::user::UserState, auth::get_current_user, db::models::user::User, DbPool};
use crate::{
    api::models::{authentication::AuthenticationChallenge, error::APIError},
    auth::is_authenticated,
    crypto,
    db::models::authentication_challenge::{
        AuthenticationChallenge as DBAuthenticationChallenge, NewAuthenticationChallenge,
    },
    settings,
};

#[derive(Deserialize)]
pub struct Info {
    address: String,
}

pub async fn sign_in(
    pool: web::Data<DbPool>,
    query: Query<Info>,
    server_settings: web::Data<settings::Server>,
    session: Session,
) -> Result<HttpResponse, APIError> {
    if is_authenticated(&session) {
        return Ok(HttpResponse::Ok().status(StatusCode::NO_CONTENT).finish());
    }

    let address = query.address.clone();
    let conn = pool.get()?;
    let users = web::block(move || {
        User::get_all(&conn, None, None, Some(UserState::Active), Some(&address))
    })
    .await?;

    if users.is_empty() {
        return Err(APIError::Forbidden);
    }

    let address = query.address.clone();
    let new_authentication_challenge = NewAuthenticationChallenge {
        address,
        challenge: format!(
            "sign-in-challenge:{};{};{}",
            server_settings.domain_name,
            chrono::Utc::now(),
            bs58::encode(crypto::generate_random_bytes(10)).into_string()
        ),
    };

    let conn = pool.get()?;
    let db_authentication_challenge = web::block(move || {
        let _ = DBAuthenticationChallenge::delete_expired(&conn);

        DBAuthenticationChallenge::insert(&conn, &new_authentication_challenge)
    })
    .await?;

    let authentication_challenge: AuthenticationChallenge = db_authentication_challenge.into();
    session.renew();
    Ok(HttpResponse::Ok().json(authentication_challenge))
}

pub async fn get_me(
    session: Session,
    server_settings: web::Data<settings::Server>,
) -> Result<HttpResponse, APIError> {
    let current_user = get_current_user(&session, server_settings.inactivity_timeout_seconds)?;

    Ok(HttpResponse::Ok().json(current_user))
}
