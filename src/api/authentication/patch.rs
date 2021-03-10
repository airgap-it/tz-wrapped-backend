use actix_session::Session;
use actix_web::{web, HttpResponse};

use crate::{
    api::models::user::{AuthUser, UserState},
    DbPool,
};
use crate::{
    api::models::{error::APIError, user::PatchAuthUser},
    auth::get_current_user,
    db::models::user::{UpdateUser, User},
    settings,
};

pub async fn me(
    pool: web::Data<DbPool>,
    body: web::Json<PatchAuthUser>,
    session: Session,
    server_settings: web::Data<settings::Server>,
) -> Result<HttpResponse, APIError> {
    let current_user = get_current_user(&session, server_settings.inactivity_timeout_seconds)?;
    let address = current_user.address.clone();
    let conn = pool.get()?;
    let users = web::block(move || User::get_all(&conn, None, None, None, Some(&address), None))
        .await?
        .into_iter()
        .map(|user| UpdateUser {
            id: user.id,
            state: user.state,
            display_name: body.display_name.clone().unwrap_or(user.display_name),
            email: body.email.clone().or(user.email),
        })
        .collect::<Vec<UpdateUser>>();

    let conn = pool.get()?;
    let address = current_user.address.clone();
    let user = web::block(move || {
        let _ = User::update(&conn, users);
        User::get_first(&conn, &address, Some(UserState::Active), None, None)
    })
    .await?;
    Ok(HttpResponse::Ok().json(AuthUser::from(user, current_user)))
}
