use std::{
    convert::{TryFrom, TryInto},
    fmt::Display,
};

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::models::{
    operation_request::OperationRequest as DBOperationRequest, user::User as DBUser,
};

use super::{error::APIError, operation_approval::NewOperationApproval, user::User};

#[derive(Serialize, Deserialize, Debug)]
pub struct OperationRequest {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub gatekeeper: User,
    pub contract_id: Uuid,
    pub target_address: Option<String>,
    pub amount: i64,
    pub kind: OperationRequestKind,
    pub signature: String,
    pub chain_id: String,
    pub nonce: i64,
    pub state: OperationRequestState,
    pub operation_hash: Option<String>,
}

impl OperationRequest {
    pub fn from(
        operation: DBOperationRequest,
        gatekeeper: DBUser,
    ) -> Result<OperationRequest, APIError> {
        Ok(OperationRequest {
            id: operation.id,
            created_at: operation.created_at,
            updated_at: operation.updated_at,
            gatekeeper: gatekeeper.try_into()?,
            contract_id: operation.contract_id,
            target_address: operation.target_address,
            amount: operation.amount,
            kind: operation.kind.try_into()?,
            signature: operation.signature,
            chain_id: operation.chain_id,
            nonce: operation.nonce,
            state: operation.state.try_into()?,
            operation_hash: operation.operation_hash,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewOperationRequest {
    pub contract_id: Uuid,
    pub target_address: Option<String>,
    pub amount: i64,
    pub kind: OperationRequestKind,
    pub signature: String,
    pub chain_id: String,
    pub nonce: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PatchOperationRequestBody {
    pub operation_hash: String,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OperationRequestKind {
    Mint = 0,
    Burn = 1,
}

const MINT: &'static str = "mint";
const BURN: &'static str = "burn";

impl TryFrom<&str> for OperationRequestKind {
    type Error = APIError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            MINT => Ok(OperationRequestKind::Mint),
            BURN => Ok(OperationRequestKind::Burn),
            _ => Err(APIError::Internal {
                description: format!("invalid operation kind: {}", value),
            }),
        }
    }
}

impl TryFrom<i16> for OperationRequestKind {
    type Error = APIError;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(OperationRequestKind::Mint),
            1 => Ok(OperationRequestKind::Burn),
            _ => Err(APIError::InvalidValue {
                description: format!("operation kind cannot be {}", value),
            }),
        }
    }
}

impl Into<&'static str> for OperationRequestKind {
    fn into(self) -> &'static str {
        match self {
            OperationRequestKind::Mint => MINT,
            OperationRequestKind::Burn => BURN,
        }
    }
}

impl Into<i16> for OperationRequestKind {
    fn into(self) -> i16 {
        match self {
            OperationRequestKind::Mint => 0,
            OperationRequestKind::Burn => 1,
        }
    }
}

impl Display for OperationRequestKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value: &str = match self {
            OperationRequestKind::Mint => MINT,
            OperationRequestKind::Burn => BURN,
        };
        write!(f, "{}", value)
    }
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OperationRequestState {
    Open = 0,
    Approved = 1,
    Injected = 2,
}

const OPEN: &'static str = "open";
const APPROVED: &'static str = "approved";
const INJECTED: &'static str = "injected";

impl TryFrom<&str> for OperationRequestState {
    type Error = APIError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            OPEN => Ok(OperationRequestState::Open),
            APPROVED => Ok(OperationRequestState::Approved),
            INJECTED => Ok(OperationRequestState::Injected),
            _ => Err(APIError::InvalidValue {
                description: format!("operation state cannot be {}", value),
            }),
        }
    }
}

impl TryFrom<i16> for OperationRequestState {
    type Error = APIError;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(OperationRequestState::Open),
            1 => Ok(OperationRequestState::Approved),
            2 => Ok(OperationRequestState::Injected),
            _ => Err(APIError::InvalidValue {
                description: format!("operation state cannot be {}", value),
            }),
        }
    }
}

impl Into<&'static str> for OperationRequestState {
    fn into(self) -> &'static str {
        match self {
            OperationRequestState::Open => OPEN,
            OperationRequestState::Approved => APPROVED,
            OperationRequestState::Injected => INJECTED,
        }
    }
}

impl Into<i16> for OperationRequestState {
    fn into(self) -> i16 {
        match self {
            OperationRequestState::Open => 0,
            OperationRequestState::Approved => 1,
            OperationRequestState::Injected => 2,
        }
    }
}

impl Display for OperationRequestState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value: &'static str = match self {
            OperationRequestState::Open => OPEN,
            OperationRequestState::Approved => APPROVED,
            OperationRequestState::Injected => INJECTED,
        };
        write!(f, "{}", value)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApprovableOperationRequest {
    pub unsigned_operation_approval: NewOperationApproval,
    pub signable_message: String,
}