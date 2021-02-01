use std::convert::TryInto;

use actix_session::Session;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::models::{error::APIError, user::UserKind},
    db::models::user::User,
};

const CURRENT_USER_KEY: &str = "current_user";

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionUser {
    pub address: String,
    pub roles: Vec<SessionUserRole>,
}

impl SessionUser {
    pub fn new(address: String, associated_db_users: Vec<User>) -> Self {
        SessionUser {
            address,
            roles: associated_db_users
                .iter()
                .map(|user| SessionUserRole {
                    contract_id: user.contract_id,
                    kind: user.kind.try_into().unwrap(),
                })
                .collect(),
        }
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

pub fn get_current_user(session: &Session) -> Result<SessionUser, APIError> {
    let user = session
        .get::<SessionUser>(CURRENT_USER_KEY)
        .map_err(|_error| APIError::Unauthorized)?;

    if let Some(user) = user {
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
