use actix_session::Session;
use actix_web::{
    http::StatusCode,
    web::{self, Query},
    HttpResponse,
};
use serde::Deserialize;

use crate::{
    api::models::user::{AuthUser, UserState},
    auth::get_current_user,
    db::models::contract::Contract,
    db::models::user::User,
    db::sync_keyholders,
    tezos::{
        coding,
        micheline::{self, HexEncodable},
        TzError,
    },
    DbPool,
};
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
    contract_settings: web::Data<Vec<settings::Contract>>,
    tezos_settings: web::Data<settings::Tezos>,
    session: Session,
) -> Result<HttpResponse, APIError> {
    if is_authenticated(&session) {
        return Ok(HttpResponse::Ok().status(StatusCode::NO_CONTENT).finish());
    }

    let conn = pool.get()?;
    let contracts = web::block(move || Contract::get_all(&conn)).await?;

    sync_keyholders(
        &pool,
        contracts,
        &tezos_settings.node_url,
        &contract_settings,
    )
    .await?;

    let address = query.address.clone();
    let conn = pool.get()?;
    let users = web::block(move || {
        User::get_all(
            &conn,
            None,
            None,
            Some(UserState::Active),
            Some(&address),
            None,
        )
    })
    .await?;

    if users.is_empty() {
        return Err(APIError::Forbidden);
    }

    let address = query.address.clone();
    let branch = block_hash(&tezos_settings.node_url).await?;
    let forged_branch = hex::encode(coding::encode(&branch, coding::B, None)?);
    let content = micheline::bytes(
        format!(
            "Tezos Signed Message: {} {} {}",
            server_settings.domain_name,
            chrono::Utc::now(),
            bs58::encode(crypto::generate_random_bytes(10)).into_string()
        )
        .as_bytes()
        .to_vec(),
    )
    .to_hex_encoded()?
    .split_off(2);
    let new_authentication_challenge = NewAuthenticationChallenge {
        address,
        challenge: format!("03{}11{}", forged_branch, content),
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

pub async fn me(
    pool: web::Data<DbPool>,
    session: Session,
    server_settings: web::Data<settings::Server>,
) -> Result<HttpResponse, APIError> {
    let current_user = get_current_user(&session, server_settings.inactivity_timeout_seconds)?;
    let address = current_user.address.clone();
    let conn = pool.get()?;
    let user =
        web::block(move || User::get_first(&conn, &address, Some(UserState::Active), None, None))
            .await?;

    Ok(HttpResponse::Ok().json(AuthUser::from(user, current_user)))
}

async fn block_hash(node_url: &str) -> Result<String, APIError> {
    let url = format!("{}/chains/main/blocks/head/hash", node_url);

    let response = reqwest::get(&url)
        .await
        .map_err(|_error| TzError::NetworkFailure)?
        .json::<String>()
        .await
        .map_err(|_error| TzError::ParsingFailure)?;

    Ok(response)
}
