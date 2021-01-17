use std::convert::TryFrom;

use multisig::SignableMessage;
use num_bigint::BigInt;

use self::multisig::Multisig;
use super::micheline::MichelsonV1Expression;
use crate::{
    api::models::{
        contract::ContractKind, error::APIError, operation_request::OperationRequestKind,
    },
    db::models::contract::Contract,
};

pub mod fa1;
pub mod fa2;
pub mod multisig;

pub async fn get_signable_message(
    contract: &Contract,
    operation_kind: OperationRequestKind,
    target_address: Option<&String>,
    amount: BigInt,
    nonce: i64,
    chain_id: &str,
    multisig: &Box<dyn Multisig + '_>,
) -> Result<SignableMessage, APIError> {
    let call = contract_call_for(contract, operation_kind, target_address, amount)?;
    let message = multisig
        .signable_message_for_call(chain_id.into(), nonce, contract.pkh.clone(), call)
        .await?;

    Ok(message)
}

pub fn contract_call_for(
    contract: &Contract,
    operation_kind: OperationRequestKind,
    target_address: Option<&String>,
    amount: BigInt,
) -> Result<MichelsonV1Expression, APIError> {
    let contract_kind = ContractKind::try_from(contract.kind)?;
    match contract_kind {
        ContractKind::FA1 => match operation_kind {
            OperationRequestKind::Mint => match target_address {
                Some(target_address) => Ok(fa1::mint_call_micheline(target_address.into(), amount)),
                _ => Err(APIError::InvalidOperationRequest {
                    description: "target_address is required for mint operation requests"
                        .to_owned(),
                }),
            },
            OperationRequestKind::Burn => Ok(fa1::burn_call_micheline(amount.clone())),
        },
        ContractKind::FA2 => match operation_kind {
            OperationRequestKind::Mint => match target_address {
                Some(target_address) => Ok(fa2::mint_call_micheline(
                    target_address.into(),
                    contract.pkh.clone(),
                    amount.clone(),
                    contract.token_id.into(),
                )),
                _ => Err(APIError::InvalidOperationRequest {
                    description: "target_address is required for mint operation requests"
                        .to_owned(),
                }),
            },
            OperationRequestKind::Burn => Ok(fa2::burn_call_micheline(
                contract.pkh.clone(),
                amount.clone(),
                contract.token_id.into(),
            )),
        },
    }
}
