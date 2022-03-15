use std::convert::TryInto;

use actix_web::web;

use crate::{
    api::models::{error::APIError, user::UserKind},
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
) -> Result<(), APIError> {
    for contract in contracts {
        let mut multisig = tezos::multisig::get_multisig(
            contract.multisig_pkh.as_ref(),
            contract.kind.try_into()?,
            node_url,
        );

        let keyholders: Vec<_> = multisig
            .approvers()
            .await?
            .into_iter()
            .map(|public_key| SyncUser {
                public_key: public_key.clone(),
                display_name: "".into(),
                email: None,
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
