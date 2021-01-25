use std::convert::TryFrom;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::models::authentication_challenge::AuthenticationChallenge as DBAuthenticationChallenge;

use super::error::APIError;

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthenticationChallenge {
    pub id: Uuid,
    pub message: String,
}

impl From<DBAuthenticationChallenge> for AuthenticationChallenge {
    fn from(value: DBAuthenticationChallenge) -> Self {
        AuthenticationChallenge {
            id: value.id,
            message: value.challenge,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthenticationChallengeResponse {
    pub id: Uuid,
    pub signature: String,
}

#[derive(Debug, PartialEq)]
pub enum AuthenticationChallengeState {
    Pending = 0,
    Completed = 1,
}

impl TryFrom<i16> for AuthenticationChallengeState {
    type Error = APIError;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(AuthenticationChallengeState::Pending),
            1 => Ok(AuthenticationChallengeState::Completed),
            _ => Err(APIError::InvalidValue {
                description: format!("authentication challenge state cannot be {}", value),
            }),
        }
    }
}

impl Into<i16> for AuthenticationChallengeState {
    fn into(self) -> i16 {
        match self {
            AuthenticationChallengeState::Pending => 0,
            AuthenticationChallengeState::Completed => 1,
        }
    }
}
