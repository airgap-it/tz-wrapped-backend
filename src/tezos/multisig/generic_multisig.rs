use std::convert::TryInto;

use crate::{
    api::models::operation_request::OperationRequestKind,
    tezos::{
        self,
        micheline::{data, int, sequence, string, types},
    },
};
use crate::{
    db::models::contract::Contract,
    tezos::{micheline::MichelsonV1Expression, TzError},
};
use async_trait::async_trait;
use num_bigint::BigInt;
use tezos::micheline::{extract_key, extract_string, instructions};

use super::{
    validate, Multisig, OperationRequestParams, Parameters, SignableMessage, Signature, Storage,
};

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

    async fn signable_message(
        &self,
        contract: &Contract,
        operation_request_params: &OperationRequestParams,
        proposed_keyholders_pk: Option<Vec<String>>,
    ) -> Result<SignableMessage, TzError> {
        validate(operation_request_params, &proposed_keyholders_pk)?;

        let message =
            self.michelson_message(contract, operation_request_params, proposed_keyholders_pk);

        let data = data::pair(
            string(operation_request_params.chain_id.clone()),
            data::pair(
                string(self.address.clone()),
                data::pair(int(operation_request_params.nonce), message),
            ),
        );
        let schema = types::pair(
            types::chain_id(),
            types::pair(
                types::address(),
                types::pair(
                    types::nat(),
                    types::lambda(types::unit(), types::list(types::operation())),
                ),
            ),
        );

        Ok(SignableMessage {
            packed_data: data.pack(Some(&schema))?,
            michelson_data: data,
            michelson_type: schema,
        })
    }

    async fn transaction_parameters(
        &mut self,
        contract: &Contract,
        operation_request_params: &OperationRequestParams,
        proposed_keyholders_pk: Option<Vec<String>>,
        signatures: Vec<Signature<'_>>,
    ) -> Result<Parameters, TzError> {
        validate(operation_request_params, &proposed_keyholders_pk)?;

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

        let value = self.michelson_transaction_parameters(
            contract,
            operation_request_params,
            proposed_keyholders_pk,
            signature_map,
        );

        let operation_request_kind: OperationRequestKind =
            operation_request_params.kind.try_into().unwrap();
        let entrypoint = GenericMultisig::entrypoint(operation_request_kind);

        Ok(Parameters { entrypoint, value })
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

    fn michelson_transaction_parameters(
        &self,
        contract: &Contract,
        operation_request_params: &OperationRequestParams,
        proposed_keyholders_pk: Option<Vec<String>>,
        signature_map: MichelsonV1Expression,
    ) -> MichelsonV1Expression {
        let operation_request_kind: OperationRequestKind =
            operation_request_params.kind.try_into().unwrap();

        match operation_request_kind {
            OperationRequestKind::Mint => {
                let lambda = self.mint_lambda(
                    operation_request_params
                        .target_address
                        .as_ref()
                        .unwrap()
                        .into(),
                    contract.pkh.clone(),
                    operation_request_params
                        .amount
                        .as_ref()
                        .unwrap()
                        .as_bigint_and_exponent()
                        .0,
                    contract.token_id.into(),
                );

                data::pair(lambda, signature_map)
            }
            OperationRequestKind::Burn => {
                let lambda = self.burn_lambda(
                    contract.pkh.clone(),
                    operation_request_params
                        .amount
                        .as_ref()
                        .unwrap()
                        .as_bigint_and_exponent()
                        .0,
                    contract.token_id.into(),
                );

                data::pair(lambda, signature_map)
            }
            OperationRequestKind::UpdateKeyholders => self.update_keyholders_michelson_parameters(
                operation_request_params.threshold.unwrap(),
                proposed_keyholders_pk.unwrap(),
                signature_map,
            ),
        }
    }

    fn michelson_message(
        &self,
        contract: &Contract,
        operation_request_params: &OperationRequestParams,
        proposed_keyholders_pk: Option<Vec<String>>,
    ) -> MichelsonV1Expression {
        let operation_request_kind: OperationRequestKind =
            operation_request_params.kind.try_into().unwrap();

        match operation_request_kind {
            OperationRequestKind::Mint => self.mint_lambda(
                operation_request_params
                    .target_address
                    .as_ref()
                    .unwrap()
                    .into(),
                contract.pkh.clone(),
                operation_request_params
                    .amount
                    .as_ref()
                    .unwrap()
                    .as_bigint_and_exponent()
                    .0,
                contract.token_id.into(),
            ),
            OperationRequestKind::Burn => self.burn_lambda(
                contract.pkh.clone(),
                operation_request_params
                    .amount
                    .as_ref()
                    .unwrap()
                    .as_bigint_and_exponent()
                    .0,
                contract.token_id.into(),
            ),
            OperationRequestKind::UpdateKeyholders => self.update_keyholders_michelson_message(
                operation_request_params.threshold.unwrap(),
                proposed_keyholders_pk.unwrap(),
            ),
        }
    }

    fn mint_lambda(
        &self,
        address: String,
        contract_address: String,
        amount: BigInt,
        token_id: i64,
    ) -> MichelsonV1Expression {
        sequence(vec![
            instructions::drop(),
            instructions::nil(types::operation()),
            instructions::push(
                types::address(),
                string(format!("{}%mint", contract_address)),
            ),
            instructions::contract(types::list(types::pair(
                types::address(),
                types::pair(types::nat(), types::nat()),
            ))),
            sequence(vec![instructions::if_none(
                sequence(vec![instructions::unit(), instructions::fail_with()]),
                sequence(vec![]),
            )]),
            instructions::push(types::mutez(), int(0)),
            instructions::nil(types::pair(
                types::address(),
                types::pair(types::nat(), types::nat()),
            )),
            instructions::push(types::nat(), int(amount)),
            instructions::push(types::nat(), int(token_id)),
            instructions::pair(),
            instructions::push(types::address(), string(address)),
            instructions::pair(),
            instructions::cons(),
            instructions::transfer_tokens(),
            instructions::cons(),
        ])
        // data::left(data::right(data::right(sequence(vec![data::pair(
        //     string(address),
        //     data::pair(int(token_id), int(amount)),
        // )]))))
    }

    fn burn_lambda(
        &self,
        contract_address: String,
        amount: BigInt,
        token_id: i64,
    ) -> MichelsonV1Expression {
        sequence(vec![
            instructions::drop(),
            instructions::nil(types::operation()),
            instructions::push(
                types::address(),
                string(format!("{}%burn", contract_address)),
            ),
            instructions::contract(types::list(types::pair(types::nat(), types::nat()))),
            sequence(vec![instructions::if_none(
                sequence(vec![instructions::unit(), instructions::fail_with()]),
                sequence(vec![]),
            )]),
            instructions::push(types::mutez(), int(0)),
            instructions::nil(types::pair(types::nat(), types::nat())),
            instructions::push(types::nat(), int(amount)),
            instructions::push(types::nat(), int(token_id)),
            instructions::pair(),
            instructions::cons(),
            instructions::transfer_tokens(),
            instructions::cons(),
        ])
        // data::left(data::right(data::left(sequence(vec![data::pair(
        //     int(token_id),
        //     int(amount),
        // )]))))
    }

    fn update_keyholders_michelson_parameters(
        &self,
        threshold: i64,
        keyholders: Vec<String>,
        signature_map: MichelsonV1Expression,
    ) -> MichelsonV1Expression {
        data::pair(
            int(threshold),
            data::pair(
                sequence(
                    keyholders
                        .into_iter()
                        .map(|public_key| string(public_key))
                        .collect(),
                ),
                signature_map,
            ),
        )
    }

    fn update_keyholders_michelson_message(
        &self,
        threshold: i64,
        keyholders: Vec<String>,
    ) -> MichelsonV1Expression {
        data::pair(
            int(threshold),
            sequence(
                keyholders
                    .into_iter()
                    .map(|public_key| string(public_key))
                    .collect(),
            ),
        )
    }

    fn entrypoint(operation_request_kind: OperationRequestKind) -> String {
        match operation_request_kind {
            OperationRequestKind::Mint | OperationRequestKind::Burn => String::from("execute"),
            OperationRequestKind::UpdateKeyholders => String::from("update_signatory"),
        }
    }
}
