use std::convert::TryFrom;

use serde::{Deserialize, Serialize};

use crate::tezos::contract::multisig::SignableMessage;

use super::error::APIError;

#[derive(Serialize, Deserialize)]
pub struct ListResponse<T> {
    pub page: i64,
    pub total_pages: i64,
    pub results: Vec<T>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignableMessageInfo {
    pub message: String,
    pub tezos_client_command: String,
    pub blake2b_hash: String,
}

impl SignableMessageInfo {
    pub fn new(message: String, tezos_client_command: String, blake2b_hash: String) -> Self {
        SignableMessageInfo {
            message,
            tezos_client_command,
            blake2b_hash,
        }
    }
}

impl TryFrom<SignableMessage> for SignableMessageInfo {
    type Error = APIError;

    fn try_from(value: SignableMessage) -> Result<Self, Self::Error> {
        let ledger_blake2b_hash = value.ledger_blake2b_hash()?;
        Ok(SignableMessageInfo::new(
            value.packed_data,
            format!(
                "tezos-client hash data '{}' of type '{}'",
                value.michelson_data, value.michelson_type
            ),
            ledger_blake2b_hash,
        ))
    }
}
