use async_trait::async_trait;
use num_traits::ToPrimitive;
use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
};

use crate::{
    api::models::{contract::ContractKind, operation_request::OperationRequestKind},
    db::models::{
        contract::Contract, operation_request::NewOperationRequest,
        operation_request::OperationRequest,
    },
    tezos::micheline::{extract_prim, primitive::Primitive},
};
use crate::{
    crypto,
    tezos::{
        micheline::{extract_int, extract_sequence, primitive::Data, MichelsonV1Expression},
        TzError,
    },
};
use bigdecimal::BigDecimal;
use serde::Serialize;

use super::{coding::decode_public_key, micheline::extract_bytes};

mod generic_multisig;
mod specific_multisig;

pub fn get_multisig(address: &str, kind: ContractKind, node_url: &str) -> Box<dyn Multisig> {
    match kind {
        ContractKind::FA1 => Box::new(specific_multisig::SpecificMultisig::new(
            address.to_owned(),
            node_url.to_owned(),
        )) as Box<dyn Multisig>,
        ContractKind::FA2 => Box::new(generic_multisig::GenericMultisig::new(
            address.to_owned(),
            node_url.to_owned(),
        )) as Box<dyn Multisig>,
    }
}

#[async_trait]
pub trait Multisig: Send + Sync {
    fn node_url(&self) -> &String;
    fn address(&self) -> &String;

    async fn nonce(&mut self) -> Result<i64, TzError>;
    async fn min_signatures(&mut self) -> Result<i64, TzError>;
    async fn approvers(&mut self) -> Result<&Vec<String>, TzError>;

    async fn signable_message(
        &self,
        contract: &Contract,
        operation_request_params: &OperationRequestParams,
        proposed_keyholders_pk: Option<Vec<String>>,
    ) -> Result<SignableMessage, TzError>;

    async fn transaction_parameters(
        &mut self,
        contract: &Contract,
        operation_request_params: &OperationRequestParams,
        proposed_keyholders_pk: Option<Vec<String>>,
        signatures: Vec<Signature<'_>>,
    ) -> Result<Parameters, TzError>;
}

fn validate(
    operation_request_params: &OperationRequestParams,
    proposed_keyholders_pk: &Option<Vec<String>>,
) -> Result<(), TzError> {
    let operation_request_kind: OperationRequestKind = operation_request_params.kind.try_into()?;
    if operation_request_params.amount.is_none()
        && (operation_request_kind == OperationRequestKind::Mint
            || operation_request_kind == OperationRequestKind::Burn)
    {
        return Err(TzError::InvalidValue {
            description: "amount is required for mint and burn operation requests".to_owned(),
        });
    }

    if operation_request_params.target_address.is_none()
        && operation_request_kind == OperationRequestKind::Mint
    {
        return Err(TzError::InvalidValue {
            description: "target_address is required for mint operation requests".to_owned(),
        });
    }

    if operation_request_params.threshold.is_none()
        && operation_request_kind == OperationRequestKind::UpdateKeyholders
    {
        return Err(TzError::InvalidValue {
            description: "threshold is required for update keyholders operation requests"
                .to_owned(),
        });
    }

    if proposed_keyholders_pk.is_none()
        && operation_request_kind == OperationRequestKind::UpdateKeyholders
    {
        return Err(TzError::InvalidValue {
            description: "no keyholders provided for update keyholders operation request"
                .to_owned(),
        });
    }

    Ok(())
}

#[derive(Debug, Serialize)]
pub struct Parameters {
    pub entrypoint: String,
    pub value: MichelsonV1Expression,
}

#[derive(Debug)]
pub struct Signature<'a> {
    pub value: &'a str,
    pub public_key: &'a str,
}

#[derive(Debug)]
pub struct SignableMessage {
    pub packed_data: String,
    pub michelson_data: MichelsonV1Expression,
    pub michelson_type: MichelsonV1Expression,
}

pub struct OperationRequestParams {
    pub target_address: Option<String>,
    pub amount: Option<BigDecimal>,
    pub threshold: Option<i64>,
    pub kind: i16,
    pub chain_id: String,
    pub nonce: i64,
}

impl From<OperationRequest> for OperationRequestParams {
    fn from(value: OperationRequest) -> Self {
        OperationRequestParams {
            target_address: value.target_address,
            amount: value.amount,
            threshold: value.threshold,
            kind: value.kind,
            chain_id: value.chain_id,
            nonce: value.nonce,
        }
    }
}

impl From<NewOperationRequest> for OperationRequestParams {
    fn from(value: NewOperationRequest) -> Self {
        OperationRequestParams {
            target_address: value.target_address,
            amount: value.amount,
            threshold: value.threshold,
            kind: value.kind,
            chain_id: value.chain_id,
            nonce: value.nonce,
        }
    }
}

impl SignableMessage {
    pub fn blake2b_hash(&self) -> Result<Vec<u8>, TzError> {
        let message_bytes =
            hex::decode(&self.packed_data).map_err(|_error| TzError::HexDecodingFailure)?;

        Ok(crypto::generic_hash(&message_bytes, 32).map_err(|_error| TzError::HashFailure)?)
    }

    pub fn ledger_blake2b_hash(&self) -> Result<String, TzError> {
        Ok(bs58::encode(self.blake2b_hash()?).into_string())
    }
}

#[derive(Debug)]
struct Storage {
    nonce: i64,
    min_signatures: i64,
    approvers_public_keys: Vec<String>,
}

impl TryFrom<&MichelsonV1Expression> for Storage {
    type Error = TzError;

    fn try_from(micheline: &MichelsonV1Expression) -> Result<Self, Self::Error> {
        let mut value = extract_prim(micheline)?;

        if value.prim != Primitive::Data(Data::Pair) || value.args_count() != 2 {
            return Err(TzError::InvalidType);
        }

        let mut arguments = value.args.as_ref().unwrap();
        let nonce = extract_int(arguments.first().unwrap())?;

        value = extract_prim(arguments.last().unwrap())?;

        if value.prim != Primitive::Data(Data::Pair) || value.args_count() != 2 {
            return Err(TzError::InvalidType);
        }

        arguments = value.args.as_ref().unwrap();
        let first = arguments.first().unwrap();
        let second = arguments.last().unwrap();
        let min_signatures = extract_int(first).or_else(|_error| extract_int(second))?;
        let public_keys = extract_sequence(first)
            .or_else(|_error| extract_sequence(second))?
            .iter()
            .map(|pk| decode_public_key(extract_bytes(pk)?))
            .collect::<Result<Vec<String>, TzError>>()?;

        Ok(Storage {
            nonce: nonce.to_i64().unwrap(),
            min_signatures: min_signatures.to_i64().unwrap(),
            approvers_public_keys: public_keys.iter().map(|pk| pk.to_owned()).collect(),
        })
    }
}

impl Storage {
    async fn fetch_from(address: &String, node_url: &String) -> Result<Storage, TzError> {
        let path = format!(
            "/chains/main/blocks/head/context/contracts/{}/storage/normalized",
            address
        );
        let url = format!("{}{}", node_url, path);
        let client = reqwest::Client::new();
        let mut json = HashMap::new();
        json.insert("unparsing_mode", "Optimized_legacy");
        let response = client
            .post(&url)
            .json(&json)
            .send()
            .await
            .map_err(|_error| TzError::NetworkFailure)?
            .json::<MichelsonV1Expression>()
            .await
            .map_err(|_error| TzError::ParsingFailure)?;

        let storage = Storage::try_from(&response)?;

        Ok(storage)
    }
}
