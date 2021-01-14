use std::convert::TryFrom;

use async_trait::async_trait;

use crate::tezos::{
    micheline::{
        extract_int, extract_sequence, extract_string, primitive::Data, MichelsonV1Expression,
    },
    TzError,
};
use crate::{
    api::models::contract::ContractKind,
    tezos::micheline::{extract_prim, primitive::Primitive},
};
use serde::{Deserialize, Serialize};

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
        counter: i64,
        contract_address: String,
        call: MichelsonV1Expression,
    ) -> Result<String, TzError>;

    async fn parameters_for_call(
        &mut self,
        call: MichelsonV1Expression,
        nonce: i64,
        signatures: Vec<Signature<'_>>,
        contract_address: &str,
    ) -> Result<Parameters, TzError>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Parameters {
    pub entrypoint: String,
    pub value: MichelsonV1Expression,
}

pub struct Signature<'a> {
    pub value: &'a str,
    pub public_key: &'a str,
}

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
            nonce: *nonce,
            min_signatures: *min_signatures,
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

// #[cfg(test)]
// mod test {
//     use crate::tezos::micheline::TzError;
//     use super::*;

//     #[actix_rt::test]
//     async fn test_multisig_fetch() -> Result<(), TzError> {

//         let mut multisig = Multisig {
//             address: String::from("KT1BYMvJoM75JyqFbsLKouqkAv8dgEvViioP"),
//             node_url: String::from("https://delphinet-tezos.giganode.io"),
//             storage: None
//         };

//         let counter = multisig.counter().await?;
//         let min_sign = multisig.min_signatures().await?;
//         let pks = multisig.approvers().await?;

//         Ok(())
//     }
// }
