use std::convert::{TryFrom, TryInto};

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::APIError;
use crate::{
    auth::{SessionUser, SessionUserRole},
    db::models::user::User as DBUser,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthUser {
    pub address: String,
    pub display_name: String,
    pub email: Option<String>,
    pub roles: Vec<SessionUserRole>,
}

impl AuthUser {
    pub fn from(db_user: DBUser, session_user: SessionUser) -> AuthUser {
        AuthUser {
            address: db_user.address,
            display_name: db_user.display_name,
            email: db_user.email,
            roles: session_user.roles,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PatchAuthUser {
    pub display_name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub public_key: String,
    pub address: String,
    pub contract_id: Uuid,
    pub kind: UserKind,
    pub state: UserState,
    pub display_name: String,
}

impl TryFrom<DBUser> for User {
    type Error = APIError;

    fn try_from(value: DBUser) -> Result<Self, Self::Error> {
        Ok(User {
            id: value.id,
            created_at: value.created_at,
            updated_at: value.updated_at,
            public_key: value.public_key,
            address: value.address,
            contract_id: value.contract_id,
            kind: value.kind.try_into()?,
            state: value.state.try_into()?,
            display_name: value.display_name,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum UserKind {
    Gatekeeper = 0,
    Keyholder = 1,
    Admin = 2,
}

const GATEKEEPER: &'static str = "gatekeeper";
const KEYHOLDER: &'static str = "keyholder";
const ADMIN: &'static str = "admin";

impl TryFrom<&str> for UserKind {
    type Error = APIError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            GATEKEEPER => Ok(UserKind::Gatekeeper),
            KEYHOLDER => Ok(UserKind::Keyholder),
            ADMIN => Ok(UserKind::Admin),
            _ => Err(APIError::InvalidValue {
                description: format!("user kind cannot be {}", value),
            }),
        }
    }
}

impl TryFrom<i16> for UserKind {
    type Error = APIError;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(UserKind::Gatekeeper),
            1 => Ok(UserKind::Keyholder),
            2 => Ok(UserKind::Admin),
            _ => Err(APIError::InvalidValue {
                description: format!("user kind cannot be {}", value),
            }),
        }
    }
}

impl Into<&'static str> for UserKind {
    fn into(self) -> &'static str {
        match self {
            UserKind::Gatekeeper => GATEKEEPER,
            UserKind::Keyholder => KEYHOLDER,
            UserKind::Admin => ADMIN,
        }
    }
}

impl Into<i64> for UserKind {
    fn into(self) -> i64 {
        match self {
            UserKind::Gatekeeper => 0,
            UserKind::Keyholder => 1,
            UserKind::Admin => 2,
        }
    }
}

impl Into<i16> for UserKind {
    fn into(self) -> i16 {
        match self {
            UserKind::Gatekeeper => 0,
            UserKind::Keyholder => 1,
            UserKind::Admin => 2,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum UserState {
    Active = 0,
    Inactive = 1,
}

const ACTIVE: &'static str = "active";
const INACTIVE: &'static str = "inactive";

impl TryFrom<&str> for UserState {
    type Error = APIError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            ACTIVE => Ok(UserState::Active),
            INACTIVE => Ok(UserState::Inactive),
            _ => Err(APIError::InvalidValue {
                description: format!("user state cannot be {}", value),
            }),
        }
    }
}

impl TryFrom<i16> for UserState {
    type Error = APIError;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(UserState::Active),
            1 => Ok(UserState::Inactive),
            _ => Err(APIError::InvalidValue {
                description: format!("user state cannot be {}", value),
            }),
        }
    }
}

impl Into<&'static str> for UserState {
    fn into(self) -> &'static str {
        match self {
            UserState::Active => ACTIVE,
            UserState::Inactive => INACTIVE,
        }
    }
}

impl Into<i64> for UserState {
    fn into(self) -> i64 {
        match self {
            UserState::Active => 0,
            UserState::Inactive => 1,
        }
    }
}

impl Into<i16> for UserState {
    fn into(self) -> i16 {
        match self {
            UserState::Active => 0,
            UserState::Inactive => 1,
        }
    }
}
