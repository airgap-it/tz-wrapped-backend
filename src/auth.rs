use std::convert::TryInto;

use actix_session::Session;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::models::{error::APIError, user::UserKind},
    db::models::user::User,
};

const CURRENT_USER_KEY: &str = "current_user";
const LAST_ACTIVITY: &str = "last_activity";

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionUser {
    pub address: String,
    pub roles: Vec<SessionUserRole>,
}

impl SessionUser {
    pub fn new(address: String, associated_db_users: &Vec<User>) -> Self {
        let roles = associated_db_users
            .iter()
            .map(|user| SessionUserRole {
                contract_id: user.contract_id,
                kind: user.kind.try_into().unwrap(),
            })
            .collect::<Vec<SessionUserRole>>();

        SessionUser { address, roles }
    }

    pub fn require_roles(&self, kinds: Vec<UserKind>, contract_id: Uuid) -> Result<(), APIError> {
        let roles: Vec<&SessionUserRole> = self
            .roles
            .iter()
            .filter(|role| role.contract_id == contract_id && kinds.contains(&role.kind))
            .collect();

        if roles.is_empty() {
            return Err(APIError::Forbidden);
        }

        Ok(())
    }

    pub fn require_one_of_roles(&self, kinds: Vec<UserKind>) -> Result<(), APIError> {
        let roles: Vec<&SessionUserRole> = self
            .roles
            .iter()
            .filter(|role| kinds.contains(&role.kind))
            .collect();

        if roles.is_empty() {
            return Err(APIError::Forbidden);
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionUserRole {
    pub contract_id: Uuid,
    pub kind: UserKind,
}

pub fn is_authenticated(session: &Session) -> bool {
    let user = session.get::<SessionUser>(CURRENT_USER_KEY);
    if let Ok(user) = user {
        return user.is_some();
    }
    return false;
}

pub fn get_current_user(session: &Session, activity_timeout: i64) -> Result<SessionUser, APIError> {
    let user = session
        .get::<SessionUser>(CURRENT_USER_KEY)
        .map_err(|_error| APIError::Unauthorized)?;

    if let Some(user) = user {
        let last_timestamp = get_last_activity_timestamp(session)?;
        let now = Utc::now().timestamp();

        if now - last_timestamp > activity_timeout {
            session.clear();

            return Err(APIError::Unauthorized);
        }

        let _ = update_last_activity_timestamp(session);
        return Ok(user);
    }

    Err(APIError::Unauthorized)
}

pub fn set_current_user(session: &Session, user: &SessionUser) -> Result<(), actix_web::Error> {
    session.set(CURRENT_USER_KEY, user)?;

    Ok(())
}

pub fn remove_current_user(session: &Session) -> () {
    session.remove(CURRENT_USER_KEY)
}

fn update_last_activity_timestamp(session: &Session) -> Result<(), actix_web::Error> {
    let now = Utc::now().timestamp();
    session.set(LAST_ACTIVITY, now)?;

    Ok(())
}

fn get_last_activity_timestamp(session: &Session) -> Result<i64, APIError> {
    let last_activity_timestamp = session
        .get::<i64>(LAST_ACTIVITY)
        .map_err(|_error| APIError::Unauthorized)?;

    if let Some(last_activity_timestamp) = last_activity_timestamp {
        return Ok(last_activity_timestamp);
    }

    Ok(Utc::now().timestamp())
}
