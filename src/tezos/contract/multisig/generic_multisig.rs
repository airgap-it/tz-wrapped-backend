use crate::tezos::{
    self,
    micheline::{data, int, sequence, string, types},
};
use crate::tezos::{micheline::MichelsonV1Expression, TzError};
use async_trait::async_trait;
use tezos::micheline::{extract_key, extract_string};

use super::{Multisig, Parameters, Signature, Storage};

pub struct GenericMultisig {
    address: String,
    node_url: String,

    storage: Option<Storage>,
}

#[async_trait]
impl Multisig for GenericMultisig {
    fn node_url(&self) -> &String {
        &self.node_url
    }

    fn address(&self) -> &String {
        &self.address
    }

    async fn nonce(&mut self) -> Result<i64, TzError> {
        let storage = self.fetch_storage().await?;

        Ok(storage.nonce + 1)
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
        _contract_address: String,
        call: MichelsonV1Expression,
    ) -> Result<String, TzError> {
        let micheline = data::pair(string(chain_id), data::pair(int(counter), call));
        let schema = types::pair(
            types::chain_id(),
            types::pair(
                types::nat(),
                types::lambda(types::unit(), types::operation()),
            ),
        );

        let result = micheline.pack(Some(&schema))?;

        Ok(result)
    }

    async fn parameters_for_call(
        &mut self,
        call: MichelsonV1Expression,
        _nonce: i64,
        signatures: Vec<Signature<'_>>,
        _contract_address: &str,
    ) -> Result<Parameters, TzError> {
        let mut signature_map_items = signatures
            .into_iter()
            .map(|signature| {
                let key = tezos::edpk_to_tz1(signature.public_key)?;

                Ok(data::elt(string(key), string(signature.value.to_owned())))
            })
            .collect::<Result<Vec<MichelsonV1Expression>, TzError>>()?;
        signature_map_items.sort_unstable_by(|a, b| {
            let a_key = extract_string(extract_key(a).unwrap()).unwrap();
            let b_key = extract_string(extract_key(b).unwrap()).unwrap();

            a_key.cmp(&b_key)
        });
        let signature_map = sequence(signature_map_items);

        let value = data::pair(call, signature_map);

        Ok(Parameters {
            entrypoint: "default".into(),
            value,
        })
    }
}

impl GenericMultisig {
    pub fn new(address: String, node_url: String) -> Self {
        GenericMultisig {
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
}
