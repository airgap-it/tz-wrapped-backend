use std::convert::TryFrom;

use crate::tezos::micheline::{
    bytes, data, extract_int, extract_prim, extract_sequence, extract_string, int, sequence,
    string, types,
};
use crate::tezos::{
    coding,
    micheline::{
        primitive::Data, primitive::Primitive, primitive::Type, MichelsonV1Expression, TzError,
    },
};
use serde::{Deserialize, Serialize};

pub struct Multisig<'a> {
    address: &'a str,
    node_url: &'a str,

    storage: Option<Storage>,
}

impl<'a> Multisig<'a> {
    pub fn new(address: &'a str, node_url: &'a str) -> Self {
        Multisig {
            address,
            node_url,
            storage: None,
        }
    }

    pub async fn nonce(&mut self) -> Result<i64, TzError> {
        let storage = self.fetch_storage().await?;

        Ok(storage.nonce)
    }

    pub async fn min_signatures(&mut self) -> Result<i64, TzError> {
        let storage = self.fetch_storage().await?;

        Ok(storage.min_signatures)
    }

    pub async fn approvers(&mut self) -> Result<&Vec<String>, TzError> {
        let storage = self.fetch_storage().await?;

        Ok(&storage.approvers_public_keys)
    }

    pub async fn chain_id(&self) -> Result<String, TzError> {
        let url = format!("{}/chains/main/chain_id", self.node_url);
        let result = reqwest::get(&url)
            .await
            .map_err(|_error| TzError::NetworkFailure)?
            .json::<String>()
            .await
            .map_err(|_error| TzError::ParsingFailure)?;

        Ok(result)
    }

    async fn fetch_storage(&mut self) -> Result<&Storage, TzError> {
        if let Some(_) = self.storage {
            return Ok(self.storage.as_ref().unwrap());
        }

        let path = format!(
            "/chains/main/blocks/head/context/contracts/{}/storage",
            self.address
        );
        let url = format!("{}{}", self.node_url, path);
        let response = reqwest::get(&url)
            .await
            .map_err(|_error| TzError::NetworkFailure)?
            .json::<MichelsonV1Expression>()
            .await
            .map_err(|_error| TzError::ParsingFailure)?;

        let storage = Storage::try_from(&response)?;
        self.storage = Some(storage);

        Ok(self.storage.as_ref().unwrap())
    }

    async fn fetch_main_parameter_schema(&self) -> Result<MichelsonV1Expression, TzError> {
        let path = format!(
            "/chains/main/blocks/head/context/contracts/{}/entrypoints/mainParameter",
            self.address
        );
        let url = format!("{}{}", self.node_url, path);
        let response = reqwest::get(&url)
            .await
            .map_err(|_error| TzError::NetworkFailure)?
            .json::<MichelsonV1Expression>()
            .await
            .map_err(|_error| TzError::ParsingFailure)?;

        Ok(response)
    }

    pub async fn signable_message_for_call(
        &self,
        chain_id: String,
        counter: i64,
        contract_address: String,
        call: MichelsonV1Expression,
    ) -> Result<String, TzError> {
        let micheline = data::pair(
            data::pair(string(chain_id), string(self.address.to_owned())),
            data::pair(
                int(counter),
                data::left(data::pair(call, string(contract_address))),
            ),
        );

        let main_parameter_schema = self.fetch_main_parameter_schema().await?;
        let signable_schema = match &main_parameter_schema {
            MichelsonV1Expression::Prim(value) => {
                if value.prim != Primitive::Type(Type::Pair) && value.args_count() == 2 {
                    return Err(TzError::InvalidType);
                }

                Ok(value.args.as_ref().unwrap().first().unwrap())
            }
            _ => Err(TzError::InvalidType),
        }?;

        let schema = types::pair(
            types::pair(types::chain_id(), types::address()),
            signable_schema.to_owned(),
        );

        let result = micheline.pack(Some(&schema))?;

        Ok(result)
    }

    pub async fn parameters_for_call(
        &mut self,
        call: MichelsonV1Expression,
        nonce: i64,
        signatures: Vec<Signature<'a>>,
        contract_address: &str,
    ) -> Result<Parameters, TzError> {
        let ordered_signature_list = self
            .approvers()
            .await?
            .into_iter()
            .map(|public_key| {
                signatures
                    .iter()
                    .find(|signature| signature.public_key == public_key)
                    .map(|sig| coding::encode_signature(sig.value))
                    .map_or(Ok(None), |r| r.map(Some))
                    .map(|sig_bytes| {
                        if let Some(sig_bytes) = sig_bytes {
                            data::some(bytes(sig_bytes))
                        } else {
                            data::none()
                        }
                    })
            })
            .collect::<Result<Vec<MichelsonV1Expression>, TzError>>()?;
        let value = data::pair(
            data::pair(
                int(nonce),
                data::left(data::pair(
                    call,
                    bytes(coding::encode_address(contract_address, false)?),
                )),
            ),
            sequence(ordered_signature_list),
        );

        Ok(Parameters {
            entrypoint: "mainParameter".into(),
            value,
        })
    }
}

trait MultisigStorageParser {
    fn nonce(&self) -> i64;
    fn required_number_of_signatures(&self) -> i64;
    fn keyholders_public_keys(&self) -> Vec<String>;
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
        let min_signatures = extract_int(arguments.first().unwrap())?;
        let public_keys: Vec<&String> = extract_sequence(arguments.last().unwrap())?
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Parameters {
    pub entrypoint: String,
    pub value: MichelsonV1Expression,
}

pub struct Signature<'a> {
    pub value: &'a str,
    pub public_key: &'a str,
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
