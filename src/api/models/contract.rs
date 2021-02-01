use std::convert::TryFrom;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::models::contract::Contract as DBContract;

use super::error::APIError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Contract {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub pkh: String,
    pub token_id: i32,
    pub multisig_pkh: String,
    pub kind: ContractKind,
    pub display_name: String,
    pub min_approvals: i32,
    pub decimals: i32,
}

impl TryFrom<DBContract> for Contract {
    type Error = APIError;

    fn try_from(value: DBContract) -> Result<Self, Self::Error> {
        Ok(Contract {
            id: value.id,
            created_at: value.created_at,
            updated_at: value.updated_at,
            pkh: value.pkh,
            token_id: value.token_id,
            multisig_pkh: value.multisig_pkh,
            kind: ContractKind::try_from(value.kind)?,
            display_name: value.display_name,
            min_approvals: value.min_approvals,
            decimals: value.decimals,
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

impl Into<i16> for ContractKind {
    fn into(self) -> i16 {
        match self {
            ContractKind::FA1 => 0,
            ContractKind::FA2 => 1,
        }
    }
}
