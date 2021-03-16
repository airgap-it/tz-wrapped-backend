use std::convert::TryInto;

use actix_web::web;

use crate::{
    api::models::{error::APIError, user::UserKind},
    settings::Contract as ContractSettings,
    tezos, DbPool,
};

use self::models::{
    contract::Contract,
    user::{self, SyncUser},
};

pub mod models;
pub mod schema;

pub async fn sync_keyholders(
    pool: &DbPool,
    contracts: Vec<Contract>,
    node_url: &str,
    contract_settings: &Vec<ContractSettings>,
) -> Result<(), APIError> {
    for contract in contracts {
        let mut multisig = tezos::multisig::get_multisig(
            contract.multisig_pkh.as_ref(),
            contract.kind.try_into()?,
            node_url,
        );

        let contract_settings = contract_settings
            .iter()
            .find(|contract_settings| {
                contract_settings.address == contract.pkh
                    && contract_settings.multisig == contract.multisig_pkh
                    && contract_settings.token_id == (contract.token_id as i64)
            })
            .expect("corresponding contract settings must be found");

        let keyholders: Vec<_> = multisig
            .approvers()
            .await?
            .into_iter()
            .map(|public_key| {
                let keyholder_settings =
                    contract_settings
                        .keyholders
                        .as_ref()
                        .and_then(|keyholders| {
                            keyholders
                                .iter()
                                .find(|keyholder| &keyholder.public_key == public_key)
                        });

                SyncUser {
                    public_key: public_key.clone(),
                    display_name: keyholder_settings
                        .map(|kh| kh.name.clone().unwrap_or("".into()))
                        .unwrap_or("".into()),
                    email: keyholder_settings.and_then(|kh| kh.email.clone()),
                }
            })
            .collect();
        let conn = pool.get()?;
        web::block::<_, _, APIError>(move || {
            let _changes = user::User::sync_users(
                &conn,
                contract.id,
                UserKind::Keyholder,
                keyholders.as_ref(),
            )?;

            Ok(())
        })
        .await?;
    }

    Ok(())
}
