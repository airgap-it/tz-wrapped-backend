use actix_web::web;
use chrono::NaiveDateTime;
use diesel::{prelude::*, r2d2::ConnectionManager, r2d2::PooledConnection};
use uuid::Uuid;

use super::{
    capability::{Capability, NewCapability},
    operation_request::OperationRequest,
    pagination::Paginate,
};
use crate::api::models::error::APIError;
use crate::db::schema::contracts;
use crate::settings;
use crate::tezos::multisig;
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
    pub symbol: String,
    pub decimals: i32,
}

impl Contract {
    pub fn get(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: &Uuid,
    ) -> Result<Contract, diesel::result::Error> {
        contracts::dsl::contracts.find(id).first(conn)
    }

    pub fn get_with_capabilities(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: &Uuid,
    ) -> Result<(Contract, Vec<Capability>), diesel::result::Error> {
        let contract = Contract::get(conn, id)?;
        let capabilities = Capability::belonging_to(&contract).load(conn)?;

        Ok((contract, capabilities))
    }

    pub fn get_all(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
    ) -> Result<Vec<Contract>, diesel::result::Error> {
        let contracts: Vec<Contract> = contracts::dsl::contracts
            .order_by(contracts::dsl::created_at)
            .load(conn)?;
        Ok(contracts)
    }

    pub fn get_all_with_capabilities(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
    ) -> Result<Vec<(Contract, Vec<Capability>)>, diesel::result::Error> {
        let contracts: Vec<Contract> = contracts::dsl::contracts
            .order_by(contracts::dsl::created_at)
            .load(conn)?;

        let capabilities: Vec<Vec<Capability>> = Capability::belonging_to(&contracts)
            .load(conn)?
            .grouped_by(&contracts);
        let result: Vec<(Contract, Vec<Capability>)> =
            contracts.into_iter().zip(capabilities).collect();
        Ok(result)
    }

    pub fn get_list(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        page: i64,
        limit: i64,
    ) -> Result<(Vec<(Contract, Vec<Capability>)>, i64), diesel::result::Error> {
        let contracts_query = contracts::dsl::contracts
            .order_by(contracts::dsl::display_name.asc())
            .paginate(page)
            .per_page(limit);

        let (contracts, page_count) = contracts_query.load_and_count_pages::<Contract>(&conn)?;

        let capabilities: Vec<Vec<Capability>> = Capability::belonging_to(&contracts)
            .load(conn)?
            .grouped_by(&contracts);

        let result: Vec<(Contract, Vec<Capability>)> =
            contracts.into_iter().zip(capabilities).collect();

        Ok((result, page_count))
    }

    pub fn insert(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        new_contract: (NewContract, Vec<settings::Capability>),
    ) -> Result<(Contract, Vec<Capability>), diesel::result::Error> {
        let contract: Contract = diesel::insert_into(contracts::table)
            .values(&new_contract.0)
            .get_result(conn)?;
        let new_capabilities = new_contract
            .1
            .into_iter()
            .map(|cap| NewCapability {
                contract_id: contract.id,
                operation_request_kind: cap.operation_request_kind.into(),
            })
            .collect::<Vec<_>>();
        let capabilities = Capability::insert(conn, new_capabilities)?;
        Ok((contract, capabilities))
    }

    pub fn update(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        updated_contract: UpdateContract,
    ) -> Result<Contract, diesel::result::Error> {
        diesel::update(contracts::dsl::contracts.find(updated_contract.id))
            .set(updated_contract)
            .get_result(conn)
    }

    pub fn delete(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        to_remove: Vec<Uuid>,
    ) -> Result<(), diesel::result::Error> {
        diesel::delete(contracts::dsl::contracts.filter(contracts::dsl::id.eq_any(to_remove)))
            .execute(conn)?;

        Ok(())
    }

    // TODO: refactor and optimize this method
    pub async fn sync_contracts(
        pool: &DbPool,
        contracts: &Vec<settings::Contract>,
        node_url: &str,
    ) -> Result<(), APIError> {
        let conn = pool.get()?;

        let stored_contracts =
            web::block(move || Contract::get_all_with_capabilities(&conn)).await?;
        let to_remove: Vec<_> = stored_contracts
            .iter()
            .filter(|(stored_contract, _)| {
                let found = contracts.iter().find(|contract| {
                    contract.address == stored_contract.pkh
                        && contract.multisig == stored_contract.multisig_pkh
                        && contract.token_id == (stored_contract.token_id as i64)
                });
                return found.is_none();
            })
            .map(|(contract, _)| contract.id.clone())
            .collect();

        let new_contracts: Vec<_> = contracts
            .iter()
            .filter(|contract| {
                let found = stored_contracts.iter().find(|(stored_contract, _)| {
                    stored_contract.pkh == contract.address
                        && stored_contract.multisig_pkh == contract.multisig
                        && (stored_contract.token_id as i64) == contract.token_id
                });
                return found.is_none();
            })
            .collect();

        let mut to_add = Vec::<(NewContract, Vec<settings::Capability>)>::new();
        to_add.reserve(new_contracts.len());
        for contract in new_contracts {
            let mut multisig = multisig::get_multisig(&contract.multisig, contract.kind, node_url);
            let min_approvals = multisig.min_signatures().await? as i32;
            let new_contract = NewContract {
                pkh: contract.address.clone(),
                token_id: contract.token_id as i32,
                multisig_pkh: contract.multisig.clone(),
                kind: contract.kind.into(),
                display_name: contract.name.clone(),
                min_approvals,
                symbol: contract.symbol.clone(),
                decimals: contract.decimals,
            };
            to_add.push((new_contract, contract.capabilities.clone()));
        }

        let mut to_update = Vec::<UpdateContract>::new();
        let mut capabilities_to_add = Vec::<NewCapability>::new();
        let mut capabilities_to_remove = Vec::<Uuid>::new();
        let mut contracts_with_higher_threshold = Vec::<Uuid>::new();
        for contract in contracts {
            let found = stored_contracts.iter().find(|(stored_contract, _)| {
                stored_contract.pkh == contract.address
                    && stored_contract.multisig_pkh == contract.multisig
                    && (stored_contract.token_id as i64) == contract.token_id
            });

            if let Some((stored_contract, stored_capabilities)) = found {
                let mut multisig =
                    multisig::get_multisig(&contract.multisig, contract.kind, node_url);
                let min_approvals = multisig.min_signatures().await? as i32;
                let contract_kind_i16: i16 = contract.kind.into();
                let has_changes = stored_contract.display_name != contract.name
                    || stored_contract.kind != contract_kind_i16
                    || stored_contract.min_approvals != min_approvals
                    || stored_contract.decimals != contract.decimals;
                if has_changes {
                    to_update.push(UpdateContract {
                        id: stored_contract.id,
                        kind: contract.kind.into(),
                        display_name: contract.name.clone(),
                        min_approvals,
                    });
                    if stored_contract.min_approvals < min_approvals {
                        contracts_with_higher_threshold.push(stored_contract.id)
                    }
                }
                let mut new_capabilities = contract
                    .capabilities
                    .iter()
                    .filter_map(|cap| {
                        let operation_request_kind: i16 = cap.operation_request_kind.into();
                        let found = stored_capabilities.iter().find(|stored_capability| {
                            stored_capability.operation_request_kind == operation_request_kind
                        });

                        match found {
                            Some(_) => None,
                            None => Some(NewCapability {
                                contract_id: stored_contract.id,
                                operation_request_kind,
                            }),
                        }
                    })
                    .collect::<Vec<_>>();
                capabilities_to_add.append(&mut new_capabilities);
                let mut removed_capabilities = stored_capabilities
                    .iter()
                    .filter_map(|stored_capability| {
                        let found = contract.capabilities.iter().find(|cap| {
                            let operation_request_kind: i16 = cap.operation_request_kind.into();
                            operation_request_kind == stored_capability.operation_request_kind
                        });

                        match found {
                            Some(_) => None,
                            None => Some(stored_capability.id),
                        }
                    })
                    .collect::<Vec<_>>();
                capabilities_to_remove.append(&mut removed_capabilities);
            }
        }

        let conn = pool.get()?;
        web::block::<_, _, APIError>(move || {
            conn.transaction(|| {
                if !to_remove.is_empty() {
                    Contract::delete(&conn, to_remove)?;
                }

                for new_contract in to_add {
                    Contract::insert(&conn, new_contract)?;
                }

                if !to_update.is_empty() {
                    for update in to_update {
                        Contract::update(&conn, update)?;
                    }
                    for contract_id in contracts_with_higher_threshold {
                        OperationRequest::fix_approved_state(&conn, &contract_id)?;
                    }
                }

                if !capabilities_to_add.is_empty() {
                    Capability::insert(&conn, capabilities_to_add)?;
                }

                if !capabilities_to_remove.is_empty() {
                    Capability::delete(&conn, capabilities_to_remove)?;
                }

                Ok(())
            })
        })
        .await?;

        Ok(())
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
    pub symbol: String,
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

#[derive(AsChangeset, Identifiable, Debug)]
#[table_name = "contracts"]
pub struct UpdateContract {
    pub id: Uuid,
    pub kind: i16,
    pub display_name: String,
    pub min_approvals: i32,
}
