use std::convert::{TryFrom, TryInto};

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::models::contract;

use super::{error::APIError, operations::PostOperationRequestBody};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContractResponse {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub pkh: String,
    pub token_id: i32,
    pub multisig_pkh: String,
    pub kind: ContractKind,
    pub display_name: String,
}

impl TryFrom<contract::Contract> for ContractResponse {
    type Error = APIError;

    fn try_from(value: contract::Contract) -> Result<Self, Self::Error> {
        Ok(ContractResponse {
            id: value.id,
            created_at: value.created_at,
            updated_at: value.updated_at,
            pkh: value.pkh,
            token_id: value.token_id,
            multisig_pkh: value.multisig_pkh,
            kind: value.kind.try_into()?,
            display_name: value.display_name,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ContractKind {
    FA1 = 0,
    FA2 = 1,
}

impl TryFrom<i16> for ContractKind {
    type Error = APIError;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ContractKind::FA1),
            1 => Ok(ContractKind::FA2),
            _ => Err(APIError::InvalidValue {
                description: format!("contract kind cannot be {}", value),
            }),
        }
    }
}

impl TryFrom<&str> for ContractKind {
    type Error = APIError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_ref() {
            "fa1" => Ok(ContractKind::FA1),
            "fa2" => Ok(ContractKind::FA2),
            _ => Err(APIError::InvalidValue {
                description: format!("contract kind cannot be {}", value),
            }),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContractSignableOperation {
    pub operation_request: PostOperationRequestBody,
    pub signable_message: String,
}
