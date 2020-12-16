use std::convert::TryFrom;

use multisig::Multisig;

use crate::{
    api::models::{contracts::ContractKind, error::APIError, operations::OperationKind},
    db::models::contract::Contract,
};

use super::micheline::{
    prim::Prim,
    primitive::{Data, Primitive},
    MichelsonV1Expression,
};

pub mod fa1;
pub mod multisig;

pub async fn get_signable_message<'a>(
    contract: &'a Contract,
    operation_kind: OperationKind,
    target_address: Option<&String>,
    amount: i64,
    nonce: i64,
    chain_id: &str,
    multisig: &Multisig<'a>,
) -> Result<String, APIError> {
    let call = contract_call_for(contract, operation_kind, target_address, amount)?;
    let message = multisig
        .signable_message_for_call(chain_id.into(), nonce, contract.pkh.clone(), call)
        .await?;

    Ok(message)
}

pub fn contract_call_for<'a>(
    contract: &'a Contract,
    operation_kind: OperationKind,
    target_address: Option<&String>,
    amount: i64,
) -> Result<MichelsonV1Expression, APIError> {
    let contract_kind = ContractKind::try_from(contract.kind)?;
    match contract_kind {
        ContractKind::FA1 => match operation_kind {
            OperationKind::Mint => match target_address {
                Some(target_address) => Ok(fa1::mint_call_micheline(target_address.into(), amount)),
                _ => Err(APIError::InvalidOperationRequest {
                    description: "target_address is required for mint operation requests"
                        .to_owned(),
                }),
            },
            OperationKind::Burn => Ok(fa1::burn_call_micheline(amount)),
        },
        ContractKind::FA2 => Ok(MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Unit),
            None,
            None,
        ))),
    }
}
