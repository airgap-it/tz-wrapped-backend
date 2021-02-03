use actix_web::web;
use chrono::NaiveDateTime;
use diesel::{prelude::*, r2d2::ConnectionManager, r2d2::PooledConnection};
use uuid::Uuid;

use super::pagination::Paginate;
use crate::api::models::error::APIError;
use crate::db::schema::contracts;
use crate::settings;
use crate::tezos::contract::multisig;
use crate::DbPool;

#[derive(Queryable, Identifiable, Clone, Debug)]
pub struct Contract {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub pkh: String,
    pub token_id: i32,
    pub multisig_pkh: String,
    pub kind: i16,
    pub display_name: String,
    pub min_approvals: i32,
    pub decimals: i32,
}

impl Contract {
    pub fn get(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: &Uuid,
    ) -> Result<Contract, diesel::result::Error> {
        let result: Contract = contracts::dsl::contracts.find(id).first(conn)?;

        Ok(result)
    }

    pub fn get_all(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
    ) -> Result<Vec<Contract>, diesel::result::Error> {
        let result: Vec<Contract> = contracts::dsl::contracts
            .order_by(contracts::dsl::created_at)
            .load(conn)?;

        Ok(result)
    }

    pub fn get_list(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        page: i64,
        limit: i64,
    ) -> Result<(Vec<Contract>, i64), diesel::result::Error> {
        let contracts_query = contracts::dsl::contracts
            .order_by(contracts::dsl::display_name.asc())
            .paginate(page)
            .per_page(limit);

        contracts_query.load_and_count_pages::<Contract>(&conn)
    }

    // TODO: refactor and optimize this method
    pub async fn sync_contracts(
        pool: &DbPool,
        contracts: &Vec<settings::Contract>,
        node_url: &str,
    ) -> Result<usize, APIError> {
        let conn = pool.get()?;

        let stored_contracts = web::block(move || Contract::get_all(&conn)).await?;
        let to_remove: Vec<_> = stored_contracts
            .iter()
            .filter(|stored_contract| {
                let found = contracts.iter().find(|contract| {
                    contract.address == stored_contract.pkh
                        && contract.token_id == (stored_contract.token_id as i64)
                });
                if let None = found {
                    true
                } else {
                    false
                }
            })
            .map(|contract| contract.id.clone())
            .collect();

        let new_contracts: Vec<_> = contracts
            .iter()
            .filter(|contract| {
                let found = stored_contracts.iter().find(|stored_contract| {
                    stored_contract.pkh == contract.address
                        && (stored_contract.token_id as i64) == contract.token_id
                });

                if let None = found {
                    true
                } else {
                    false
                }
            })
            .collect();

        let mut to_add = Vec::<NewContract>::new();
        to_add.reserve(new_contracts.len());
        for contract in new_contracts {
            let mut multisig = multisig::get_multisig(&contract.multisig, contract.kind, node_url);
            let min_approvals = multisig.min_signatures().await? as i32;
            to_add.push(NewContract {
                pkh: contract.address.clone(),
                token_id: contract.token_id as i32,
                multisig_pkh: contract.multisig.clone(),
                kind: contract.kind.into(),
                display_name: contract.name.clone(),
                min_approvals,
                decimals: contract.decimals,
            })
        }

        let mut to_update = Vec::<UpdateContract>::new();
        for contract in contracts {
            let found = stored_contracts.iter().find(|stored_contract| {
                stored_contract.pkh == contract.address
                    && (stored_contract.token_id as i64) == contract.token_id
            });

            if let Some(stored_contract) = found {
                let mut multisig =
                    multisig::get_multisig(&contract.multisig, contract.kind, node_url);
                let min_approvals = multisig.min_signatures().await? as i32;
                let contract_kind_i16: i16 = contract.kind.into();
                let has_changes = stored_contract.multisig_pkh != contract.multisig
                    || stored_contract.display_name != contract.name
                    || stored_contract.kind != contract_kind_i16
                    || stored_contract.min_approvals != min_approvals
                    || stored_contract.decimals != contract.decimals;
                if has_changes {
                    to_update.push(UpdateContract {
                        id: stored_contract.id,
                        multisig_pkh: contract.multisig.clone(),
                        kind: contract.kind.into(),
                        display_name: contract.name.clone(),
                        min_approvals,
                    })
                }
            }
        }

        let conn = pool.get()?;
        let changes = web::block::<_, _, APIError>(move || {
            let mut changes: usize = 0;

            if !to_remove.is_empty() {
                let deactivated = diesel::delete(
                    contracts::dsl::contracts.filter(contracts::dsl::id.eq_any(to_remove)),
                )
                .execute(&conn)?;

                changes += deactivated;
            }

            if !to_add.is_empty() {
                let added = diesel::insert_into(contracts::dsl::contracts)
                    .values(to_add)
                    .execute(&conn)?;

                changes += added;
            }

            if !to_update.is_empty() {
                for update in to_update {
                    changes += diesel::update(contracts::dsl::contracts.find(update.id))
                        .set(update)
                        .execute(&conn)?;
                }
            }

            Ok(changes)
        })
        .await?;

        Ok(changes)
    }
}

#[derive(Insertable)]
#[table_name = "contracts"]
pub struct NewContract {
    pub pkh: String,
    pub token_id: i32,
    pub multisig_pkh: String,
    pub kind: i16,
    pub display_name: String,
    pub min_approvals: i32,
    pub decimals: i32,
}

impl NewContract {
    pub fn save(
        &self,
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
    ) -> Result<Contract, diesel::result::Error> {
        diesel::insert_into(contracts::dsl::contracts)
            .values(self)
            .get_result::<Contract>(conn)
    }
}

#[derive(AsChangeset, Debug)]
#[table_name = "contracts"]
pub struct UpdateContract {
    pub id: Uuid,
    pub multisig_pkh: String,
    pub kind: i16,
    pub display_name: String,
    pub min_approvals: i32,
}
