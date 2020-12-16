use chrono::NaiveDateTime;
use diesel::{prelude::*, r2d2::ConnectionManager, r2d2::PooledConnection};
use uuid::Uuid;

use crate::settings;
use crate::{api::models::error::APIError, db::schema::contracts};

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
}

impl Contract {
    pub fn get_by_id(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: Uuid,
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

    // TODO: refactor and optimize this method
    pub fn sync_contracts(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        contracts: &Vec<settings::Contract>,
    ) -> Result<usize, APIError> {
        let stored_contracts = Contract::get_all(conn)?;

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
            .map(|contract| &contract.id)
            .collect();

        let to_add: Vec<_> = contracts
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
            .map(|contract| {
                Ok(NewContract {
                    pkh: contract.address.clone(),
                    token_id: contract.token_id as i32,
                    multisig_pkh: contract.multisig.clone(),
                    kind: contract.kind as i16,
                    display_name: contract.name.clone(),
                })
            })
            .collect::<Result<Vec<NewContract>, APIError>>()?;

        let to_update: Vec<_> = contracts
            .iter()
            .filter_map(|contract| {
                let found = stored_contracts.iter().find(|stored_contract| {
                    stored_contract.pkh == contract.address
                        && (stored_contract.token_id as i64) == contract.token_id
                });

                if let Some(stored_contract) = found {
                    let has_changes = stored_contract.multisig_pkh != contract.multisig
                        || stored_contract.display_name != contract.name
                        || stored_contract.kind != (contract.kind as i16);
                    if has_changes {
                        Some(UpdateContract {
                            id: stored_contract.id,
                            multisig_pkh: contract.multisig.clone(),
                            kind: contract.kind as i16,
                            display_name: contract.name.clone(),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        let mut changes: usize = 0;

        if !to_remove.is_empty() {
            let deactivated = diesel::delete(
                contracts::dsl::contracts.filter(contracts::dsl::id.eq_any(to_remove)),
            )
            .execute(conn)?;

            changes += deactivated;
        }

        if !to_add.is_empty() {
            let added = diesel::insert_into(contracts::dsl::contracts)
                .values(to_add)
                .execute(conn)?;

            changes += added;
        }

        if !to_update.is_empty() {
            for update in to_update {
                changes += diesel::update(contracts::dsl::contracts.find(update.id))
                    .set(update)
                    .execute(conn)?;
            }
        }

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
}
