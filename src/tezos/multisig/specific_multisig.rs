use std::convert::TryInto;

use async_trait::async_trait;
use num_bigint::BigInt;

use crate::{
    api::models::operation_request::OperationRequestKind,
    tezos::micheline::{
        bytes,
        data::{self, unit},
        int, sequence, string, types,
    },
};
use crate::{
    db::models::contract::Contract,
    tezos::{
        coding,
        micheline::{primitive::Primitive, primitive::Type, MichelsonV1Expression},
        TzError,
    },
};

use super::{
    validate, Multisig, OperationRequestParams, Parameters, SignableMessage, Signature, Storage,
};

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

    async fn signable_message(
        &self,
        contract: &Contract,
        operation_request_params: &OperationRequestParams,
        proposed_keyholders_pk: Option<Vec<String>>,
    ) -> Result<SignableMessage, TzError> {
        validate(operation_request_params, &proposed_keyholders_pk)?;
        let call = self.michelson_transaction_parameters(
            contract,
            operation_request_params,
            proposed_keyholders_pk,
        );

        let micheline = data::pair(
            string(self.address.to_owned()),
            data::pair(int(operation_request_params.nonce), call),
        );

        let main_parameter_schema = self.fetch_main_parameter_schema().await?;
        let signable_schema = match &main_parameter_schema {
            MichelsonV1Expression::Prim(value) => {
                if value.prim != Primitive::Type(Type::Pair) || value.args_count() != 2 {
                    return Err(TzError::InvalidType);
                }

                Ok(value.args.as_ref().unwrap().first().unwrap())
            }
            _ => Err(TzError::InvalidType),
        }?;

        let schema = types::pair(types::address(), signable_schema.to_owned());

        Ok(SignableMessage {
            packed_data: micheline.pack(Some(&schema))?,
            michelson_data: micheline,
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
        let call = self.michelson_transaction_parameters(
            contract,
            operation_request_params,
            proposed_keyholders_pk,
        );

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
            data::pair(int(operation_request_params.nonce), call),
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

    fn michelson_transaction_parameters(
        &self,
        contract: &Contract,
        operation_request_params: &OperationRequestParams,
        proposed_keyholders_pk: Option<Vec<String>>,
    ) -> MichelsonV1Expression {
        let operation_request_kind: OperationRequestKind =
            operation_request_params.kind.try_into().unwrap();

        match operation_request_kind {
            OperationRequestKind::Mint => self.mint_michelson_parameters(
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
            OperationRequestKind::Burn => self.burn_michelson_parameters(
                contract.pkh.clone(),
                operation_request_params
                    .amount
                    .as_ref()
                    .unwrap()
                    .as_bigint_and_exponent()
                    .0,
                contract.token_id.into(),
            ),
            OperationRequestKind::UpdateKeyholders => self.update_keyholders_michelson_parameters(
                operation_request_params.threshold.unwrap(),
                proposed_keyholders_pk.unwrap(),
            ),
            OperationRequestKind::AddOperator => self.add_operator_michelson_parameters(
                operation_request_params
                    .target_address
                    .as_ref()
                    .unwrap()
                    .into(),
                contract.pkh.clone(),
            ),
            OperationRequestKind::RemoveOperator => self.remove_operator_michelson_parameters(
                operation_request_params
                    .target_address
                    .as_ref()
                    .unwrap()
                    .into(),
                contract.pkh.clone(),
            ),
            OperationRequestKind::SetRedeemAddress => self.set_redeem_address_michelson_parameters(
                operation_request_params
                    .target_address
                    .as_ref()
                    .unwrap()
                    .into(),
                contract.pkh.clone(),
            ),
            OperationRequestKind::TransferOwnership => self
                .transfer_ownership_michelson_parameters(
                    operation_request_params
                        .target_address
                        .as_ref()
                        .unwrap()
                        .into(),
                    contract.pkh.clone(),
                ),
            OperationRequestKind::AcceptOwnership => {
                self.accept_ownership_michelson_parameters(contract.pkh.clone())
            }
        }
    }

    fn mint_michelson_parameters(
        &self,
        address: String,
        contract_address: String,
        amount: BigInt,
        _token_id: i64,
    ) -> MichelsonV1Expression {
        let call = data::right(data::left(data::left(data::left(data::pair(
            string(address),
            int(amount),
        )))));

        data::left(data::pair(call, string(contract_address)))
    }

    fn burn_michelson_parameters(
        &self,
        contract_address: String,
        amount: BigInt,
        _token_id: i64,
    ) -> MichelsonV1Expression {
        let call = data::right(data::left(data::left(data::right(int(amount)))));
        data::left(data::pair(call, string(contract_address)))
    }

    fn add_operator_michelson_parameters(
        &self,
        address: String,
        contract_address: String,
    ) -> MichelsonV1Expression {
        let call = data::right(data::left(data::right(data::left(string(address)))));

        data::left(data::pair(call, string(contract_address)))
    }

    fn remove_operator_michelson_parameters(
        &self,
        address: String,
        contract_address: String,
    ) -> MichelsonV1Expression {
        let call = data::right(data::left(data::right(data::right(string(address)))));

        data::left(data::pair(call, string(contract_address)))
    }

    fn set_redeem_address_michelson_parameters(
        &self,
        address: String,
        contract_address: String,
    ) -> MichelsonV1Expression {
        let call = data::right(data::right(data::left(data::left(string(address)))));

        data::left(data::pair(call, string(contract_address)))
    }

    fn transfer_ownership_michelson_parameters(
        &self,
        address: String,
        contract_address: String,
    ) -> MichelsonV1Expression {
        let call = data::right(data::right(data::right(data::right(data::left(string(
            address,
        ))))));

        data::left(data::pair(call, string(contract_address)))
    }

    fn accept_ownership_michelson_parameters(
        &self,
        contract_address: String,
    ) -> MichelsonV1Expression {
        let call = data::right(data::right(data::right(data::right(data::right(unit())))));

        data::left(data::pair(call, string(contract_address)))
    }

    fn update_keyholders_michelson_parameters(
        &self,
        threshold: i64,
        keyholders: Vec<String>,
    ) -> MichelsonV1Expression {
        data::right(data::pair(
            int(threshold),
            sequence(
                keyholders
                    .into_iter()
                    .map(|public_key| string(public_key))
                    .collect(),
            ),
        ))
    }
}
