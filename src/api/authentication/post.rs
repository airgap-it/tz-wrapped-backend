use std::convert::TryInto;

use actix_session::Session;
use actix_web::{web, HttpResponse};
use log::info;

use crate::{
    api::models::{
        authentication::AuthenticationChallengeResponse, error::APIError, user::AuthUser,
    },
    auth::{set_current_user, SessionUser},
    db::models::authentication_challenge::AuthenticationChallenge,
};
use crate::{
    api::models::{authentication::AuthenticationChallengeState, user::UserState},
    crypto,
    db::models::user::User,
    DbPool,
};

pub async fn sign_in(
    pool: web::Data<DbPool>,
    body: web::Json<AuthenticationChallengeResponse>,
    session: Session,
) -> Result<HttpResponse, APIError> {
    let authentication_challenge_id = body.id;
    let conn = pool.get()?;
    let (authentication_challenge, users) = web::block::<_, _, APIError>(move || {
        let challenge = AuthenticationChallenge::get(&conn, &authentication_challenge_id)?;
        let users = User::get_all(
            &conn,
            None,
            None,
            Some(UserState::Active),
            Some(&challenge.address),
            None,
        )?;

        Ok((challenge, users))
    })
    .await?;

    let state: AuthenticationChallengeState = authentication_challenge.state.try_into()?;

    if state == AuthenticationChallengeState::Completed || users.is_empty() {
        return Err(APIError::Forbidden);
    }

    let user = users.first().unwrap();
    let challenge_bytes =
        hex::decode(authentication_challenge.challenge).map_err(|_error| APIError::Internal {
            description: "failed to decode challenge".into(),
        })?;
    let hashed =
        crypto::generic_hash(&challenge_bytes, 32).map_err(|_error| APIError::Internal {
            description: "failed to hash challenge".into(),
        })?;
    let verified = user.verify_message(&hashed, &body.signature)?;

    if !verified {
        return Err(APIError::InvalidSignature);
    }

    let conn = pool.get()?;
    web::block(move || {
        AuthenticationChallenge::mark_completed(&conn, &authentication_challenge_id)
    })
    .await?;

    let session_user = SessionUser::new(authentication_challenge.address.to_owned(), &users);
    set_current_user(&session, &session_user).map_err(|_error| APIError::Internal {
        description: "failed to set current user".into(),
    })?;

    info!("Signed in user: {:?}", session_user);

    Ok(HttpResponse::Ok().json(AuthUser::from(user.to_owned(), session_user)))
}
