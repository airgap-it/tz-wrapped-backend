use async_trait::async_trait;
use num_traits::ToPrimitive;
use std::convert::TryFrom;

use crate::{
    api::models::contract::ContractKind,
    tezos::micheline::{extract_prim, primitive::Primitive},
};
use crate::{
    crypto,
    tezos::{
        micheline::{
            extract_int, extract_sequence, extract_string, primitive::Data, MichelsonV1Expression,
        },
        TzError,
    },
};
use serde::Serialize;

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

    async fn chain_id(&self) -> Result<String, TzError> {
        let url = format!("{}/chains/main/chain_id", self.node_url());
        let result = reqwest::get(&url)
            .await
            .map_err(|_error| TzError::NetworkFailure)?
            .json::<String>()
            .await
            .map_err(|_error| TzError::ParsingFailure)?;

        Ok(result)
    }

    async fn signable_message_for_call(
        &self,
        chain_id: String,
        nonce: i64,
        contract_address: String,
        call: MichelsonV1Expression,
    ) -> Result<SignableMessage, TzError>;

    async fn parameters_for_call(
        &mut self,
        call: MichelsonV1Expression,
        nonce: i64,
        signatures: Vec<Signature<'_>>,
        contract_address: &str,
    ) -> Result<Parameters, TzError>;
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
        let public_keys: Vec<&String> = extract_sequence(first)
            .or_else(|_error| extract_sequence(second))?
            .iter()
            .map(|pk| extract_string(pk))
            .collect::<Result<Vec<&String>, TzError>>()?;

        Ok(Storage {
            nonce: nonce.to_i64().unwrap(),
            min_signatures: min_signatures.to_i64().unwrap(),
            approvers_public_keys: public_keys
                .iter()
                .map(|pk| pk.to_owned().to_owned())
                .collect(),
        })
    }
}

impl Storage {
    async fn fetch_from(address: &String, node_url: &String) -> Result<Storage, TzError> {
        let path = format!(
            "/chains/main/blocks/head/context/contracts/{}/storage",
            address
        );
        let url = format!("{}{}", node_url, path);
        let response = reqwest::get(&url)
            .await
            .map_err(|_error| TzError::NetworkFailure)?
            .json::<MichelsonV1Expression>()
            .await
            .map_err(|_error| TzError::ParsingFailure)?;

        let storage = Storage::try_from(&response)?;

        Ok(storage)
    }
}
