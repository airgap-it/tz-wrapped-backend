use std::{
    convert::{TryFrom, TryInto},
    fmt::Display,
};

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::models::{
    operation_approval::OperationApproval as DBOperationApproval,
    operation_request::OperationRequest as DBOperationRequest, user::User as DBUser,
};

use super::error::APIError;
use super::user::User;
use super::{common::SignableMessageInfo, operation_approval::OperationApproval};

#[derive(Serialize, Deserialize, Debug)]
pub struct OperationRequest {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub user: User,
    pub contract_id: Uuid,
    pub target_address: Option<String>,
    pub amount: Option<String>,
    pub threshold: Option<i64>,
    pub proposed_keyholders: Option<Vec<User>>,
    pub kind: OperationRequestKind,
    pub chain_id: String,
    pub nonce: i64,
    pub state: OperationRequestState,
    pub operation_approvals: Vec<OperationApproval>,
    pub operation_hash: Option<String>,
}

impl OperationRequest {
    pub fn from(
        operation_request: DBOperationRequest,
        gatekeeper: DBUser,
        operation_approvals: Vec<(DBOperationApproval, DBUser)>,
        proposed_keyholders: Option<Vec<DBUser>>,
    ) -> Result<OperationRequest, APIError> {
        Ok(OperationRequest {
            id: operation_request.id,
            created_at: operation_request.created_at,
            updated_at: operation_request.updated_at,
            user: gatekeeper.try_into()?,
            contract_id: operation_request.contract_id,
            target_address: operation_request.target_address,
            amount: operation_request.amount.map(|amount| amount.to_string()),
            threshold: operation_request.threshold,
            proposed_keyholders: proposed_keyholders
                .and_then(|keyholders| {
                    keyholders
                        .into_iter()
                        .map(|keyholder| {
                            let keyholder: Result<User, APIError> = keyholder.try_into();

                            Some(keyholder)
                        })
                        .collect::<Option<Result<Vec<User>, APIError>>>()
                })
                .map_or(Ok(None), |r| r.map(Some))?,
            kind: operation_request.kind.try_into()?,
            chain_id: operation_request.chain_id,
            nonce: operation_request.nonce,
            state: operation_request.state.try_into()?,
            operation_approvals: operation_approvals
                .into_iter()
                .map(|(operation_approval, keyholder)| {
                    OperationApproval::from(operation_approval, keyholder)
                })
                .collect::<Result<Vec<OperationApproval>, APIError>>()?,
            operation_hash: operation_request.operation_hash,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewOperationRequest {
    pub contract_id: Uuid,
    pub target_address: Option<String>,
    pub amount: Option<String>,
    pub threshold: Option<i64>,
    pub proposed_keyholders: Option<Vec<String>>,
    pub kind: OperationRequestKind,
    pub ledger_hash: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PatchOperationRequest {
    pub operation_hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OperationRequestKind {
    Mint = 0,
    Burn = 1,
    UpdateKeyholders = 2,
    AddOperator = 3,
    RemoveOperator = 4,
    SetRedeemAddress = 5,
    TransferOwnership = 6,
    AcceptOwnership = 7,
}

const MINT: &'static str = "mint";
const BURN: &'static str = "burn";
const UPDATE_KEYHOLDERS: &'static str = "update_keyholders";
const ADD_OPERATOR: &'static str = "add_operator";
const REMOVE_OPERATOR: &'static str = "remove_operator";
const SET_REDEEM_ADDRESS: &'static str = "set_redeem_address";
const TRANSFER_OWNERSHIP: &'static str = "transfer_ownership";
const ACCEPT_OWNERSHIP: &'static str = "accept_ownership";

impl TryFrom<&str> for OperationRequestKind {
    type Error = APIError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            MINT => Ok(OperationRequestKind::Mint),
            BURN => Ok(OperationRequestKind::Burn),
            UPDATE_KEYHOLDERS => Ok(OperationRequestKind::UpdateKeyholders),
            ADD_OPERATOR => Ok(OperationRequestKind::AddOperator),
            REMOVE_OPERATOR => Ok(OperationRequestKind::RemoveOperator),
            SET_REDEEM_ADDRESS => Ok(OperationRequestKind::SetRedeemAddress),
            TRANSFER_OWNERSHIP => Ok(OperationRequestKind::TransferOwnership),
            ACCEPT_OWNERSHIP => Ok(OperationRequestKind::AcceptOwnership),
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
            2 => Ok(OperationRequestKind::UpdateKeyholders),
            3 => Ok(OperationRequestKind::AddOperator),
            4 => Ok(OperationRequestKind::RemoveOperator),
            5 => Ok(OperationRequestKind::SetRedeemAddress),
            6 => Ok(OperationRequestKind::TransferOwnership),
            7 => Ok(OperationRequestKind::AcceptOwnership),
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
            OperationRequestKind::UpdateKeyholders => UPDATE_KEYHOLDERS,
            OperationRequestKind::AddOperator => ADD_OPERATOR,
            OperationRequestKind::RemoveOperator => REMOVE_OPERATOR,
            OperationRequestKind::SetRedeemAddress => SET_REDEEM_ADDRESS,
            OperationRequestKind::TransferOwnership => TRANSFER_OWNERSHIP,
            OperationRequestKind::AcceptOwnership => ACCEPT_OWNERSHIP,
        }
    }
}

impl Into<i16> for OperationRequestKind {
    fn into(self) -> i16 {
        match self {
            OperationRequestKind::Mint => 0,
            OperationRequestKind::Burn => 1,
            OperationRequestKind::UpdateKeyholders => 2,
            OperationRequestKind::AddOperator => 3,
            OperationRequestKind::RemoveOperator => 4,
            OperationRequestKind::SetRedeemAddress => 5,
            OperationRequestKind::TransferOwnership => 6,
            OperationRequestKind::AcceptOwnership => 7,
        }
    }
}

impl Display for OperationRequestKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value: &str = match self {
            OperationRequestKind::Mint => "Mint",
            OperationRequestKind::Burn => "Burn",
            OperationRequestKind::UpdateKeyholders => "Multi Signature Update",
            OperationRequestKind::AddOperator => "Add Operator",
            OperationRequestKind::RemoveOperator => "Remove Operator",
            OperationRequestKind::SetRedeemAddress => "Set Redeem Address",
            OperationRequestKind::TransferOwnership => "Transfer Ownership",
            OperationRequestKind::AcceptOwnership => "Accept Ownership",
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
pub struct SignableOperationRequest {
    pub unsigned_operation_request: NewOperationRequest,
    pub signable_message_info: SignableMessageInfo,
}

impl SignableOperationRequest {
    pub fn new(
        unsigned_operation_request: NewOperationRequest,
        signable_message_info: SignableMessageInfo,
    ) -> Self {
        SignableOperationRequest {
            unsigned_operation_request,
            signable_message_info,
        }
    }
}
