use std::convert::{TryFrom, TryInto};

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::models::{operation_request::OperationRequest, user::User};

use super::{approvals::PostOperationApprovalBody, error::APIError, users::UserResponse};

#[derive(Serialize, Deserialize, Debug)]
pub struct OperationRequestResponse {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub requester: UserResponse,
    pub contract_id: Uuid,
    pub target_address: Option<String>,
    pub amount: i64,
    pub kind: OperationKind,
    pub gk_signature: String,
    pub chain_id: String,
    pub nonce: i64,
    pub state: OperationState,
    pub operation_hash: Option<String>,
}

impl OperationRequestResponse {
    pub fn from(
        operation: OperationRequest,
        gatekeeper: User,
    ) -> Result<OperationRequestResponse, APIError> {
        Ok(OperationRequestResponse {
            id: operation.id,
            created_at: operation.created_at,
            updated_at: operation.updated_at,
            requester: gatekeeper.try_into()?,
            contract_id: operation.destination,
            target_address: operation.target_address,
            amount: operation.amount,
            kind: operation.kind.try_into()?,
            gk_signature: operation.gk_signature,
            chain_id: operation.chain_id,
            nonce: operation.nonce,
            state: operation.state.try_into()?,
            operation_hash: operation.operation_hash,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PostOperationRequestBody {
    pub destination: Uuid,
    pub target_address: Option<String>,
    pub amount: i64,
    pub kind: OperationKind,
    pub gk_signature: String,
    pub chain_id: String,
    pub nonce: i64,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OperationKind {
    Mint = 0,
    Burn = 1,
}

const MINT: &'static str = "mint";
const BURN: &'static str = "burn";

impl TryFrom<&str> for OperationKind {
    type Error = APIError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            MINT => Ok(OperationKind::Mint),
            BURN => Ok(OperationKind::Burn),
            _ => Err(APIError::Internal {
                description: format!("invalid operation kind: {}", value),
            }),
        }
    }
}

impl TryFrom<i16> for OperationKind {
    type Error = APIError;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(OperationKind::Mint),
            1 => Ok(OperationKind::Burn),
            _ => Err(APIError::InvalidValue {
                description: format!("operation kind cannot be {}", value),
            }),
        }
    }
}

impl Into<&'static str> for OperationKind {
    fn into(self) -> &'static str {
        match self {
            OperationKind::Mint => MINT,
            OperationKind::Burn => BURN,
        }
    }
}

impl Into<i16> for OperationKind {
    fn into(self) -> i16 {
        match self {
            OperationKind::Mint => 0,
            OperationKind::Burn => 1,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum OperationState {
    Open = 0,
    Approved = 1,
}

const OPEN: &'static str = "open";
const APPROVED: &'static str = "approved";

impl TryFrom<&str> for OperationState {
    type Error = APIError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            OPEN => Ok(OperationState::Open),
            APPROVED => Ok(OperationState::Approved),
            _ => Err(APIError::InvalidValue {
                description: format!("operation state cannot be {}", value),
            }),
        }
    }
}

impl TryFrom<i16> for OperationState {
    type Error = APIError;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(OperationState::Open),
            1 => Ok(OperationState::Approved),
            _ => Err(APIError::InvalidValue {
                description: format!("operation state cannot be {}", value),
            }),
        }
    }
}

impl Into<&'static str> for OperationState {
    fn into(self) -> &'static str {
        match self {
            OperationState::Open => OPEN,
            OperationState::Approved => APPROVED,
        }
    }
}

impl Into<i16> for OperationState {
    fn into(self) -> i16 {
        match self {
            OperationState::Open => 0,
            OperationState::Approved => 1,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApprovableOperation {
    pub operation_approval: PostOperationApprovalBody,
    pub signable_message: String,
}
