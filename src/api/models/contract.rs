use std::convert::{TryFrom, TryInto};

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::models::{capability::Capability, contract::Contract as DBContract};

use super::{error::APIError, operation_request::OperationRequestKind};

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
    pub symbol: String,
    pub decimals: i32,
    pub capabilities: Vec<OperationRequestKind>,
}

impl TryFrom<(DBContract, Vec<Capability>)> for Contract {
    type Error = APIError;

    fn try_from(value: (DBContract, Vec<Capability>)) -> Result<Self, Self::Error> {
        let (contract, capabilities) = value;
        Ok(Contract {
            id: contract.id,
            created_at: contract.created_at,
            updated_at: contract.updated_at,
            pkh: contract.pkh,
            token_id: contract.token_id,
            multisig_pkh: contract.multisig_pkh,
            kind: ContractKind::try_from(contract.kind)?,
            display_name: contract.display_name,
            min_approvals: contract.min_approvals,
            symbol: contract.symbol,
            decimals: contract.decimals,
            capabilities: capabilities
                .iter()
                .map(|cap| cap.operation_request_kind.try_into())
                .collect::<Result<Vec<OperationRequestKind>, APIError>>()?,
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
