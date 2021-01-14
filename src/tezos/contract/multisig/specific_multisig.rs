use async_trait::async_trait;

use crate::tezos::micheline::{bytes, data, int, sequence, string, types};
use crate::tezos::{
    coding,
    micheline::{primitive::Primitive, primitive::Type, MichelsonV1Expression},
    TzError,
};

use super::{Multisig, Parameters, Signature, Storage};

pub struct SpecificMultisig {
    address: String,
    node_url: String,

    storage: Option<Storage>,
}

#[async_trait]
impl Multisig for SpecificMultisig {
    fn node_url(&self) -> &String {
        &self.node_url
    }

    fn address(&self) -> &String {
        &self.address
    }

    async fn nonce(&mut self) -> Result<i64, TzError> {
        let storage = self.fetch_storage().await?;

        Ok(storage.nonce)
    }

    async fn min_signatures(&mut self) -> Result<i64, TzError> {
        let storage = self.fetch_storage().await?;

        Ok(storage.min_signatures)
    }

    async fn approvers(&mut self) -> Result<&Vec<String>, TzError> {
        let storage = self.fetch_storage().await?;

        Ok(&storage.approvers_public_keys)
    }

    async fn signable_message_for_call(
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

    async fn parameters_for_call(
        &mut self,
        call: MichelsonV1Expression,
        nonce: i64,
        signatures: Vec<Signature<'_>>,
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

impl SpecificMultisig {
    pub fn new(address: String, node_url: String) -> Self {
        SpecificMultisig {
            address,
            node_url,
            storage: None,
        }
    }

    async fn fetch_storage(&mut self) -> Result<&Storage, TzError> {
        if let Some(_) = self.storage {
            return Ok(self.storage.as_ref().unwrap());
        }

        let storage = Storage::fetch_from(self.address(), self.node_url()).await?;
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
}
