use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use diesel::{prelude::*, r2d2::ConnectionManager, r2d2::PooledConnection};
use uuid::Uuid;

use crate::{
    api::models::operation_request::OperationRequestKind,
    db::schema::{contracts, operation_requests, users},
};
use crate::{
    api::models::operation_request::OperationRequestState,
    db::models::{contract::Contract, user::User},
};

use super::{operation_approval::OperationApproval, pagination::Paginate};

#[derive(Queryable, Identifiable, Associations, Debug)]
#[belongs_to(User, foreign_key = "gatekeeper_id")]
#[belongs_to(Contract, foreign_key = "contract_id")]
pub struct OperationRequest {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub gatekeeper_id: Uuid,
    pub contract_id: Uuid,
    pub target_address: Option<String>,
    pub amount: BigDecimal,
    pub kind: i16,
    pub chain_id: String,
    pub nonce: i64,
    pub state: i16,
    pub operation_hash: Option<String>,
}

impl OperationRequest {
    pub fn get(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: &Uuid,
    ) -> Result<OperationRequest, diesel::result::Error> {
        let result: OperationRequest = operation_requests::table.find(id).first(conn)?;

        Ok(result)
    }

    pub fn get_with_contract(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: &Uuid,
    ) -> Result<(OperationRequest, Contract), diesel::result::Error> {
        operation_requests::table
            .find(id)
            .inner_join(contracts::table)
            .first(conn)
    }

    pub fn mark_approved(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: &Uuid,
    ) -> Result<(), diesel::result::Error> {
        let _result = diesel::update(operation_requests::table.find(id))
            .set(operation_requests::dsl::state.eq::<i16>(OperationRequestState::Approved.into()))
            .execute(conn)?;

        Ok(())
    }

    pub fn mark_injected(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: &Uuid,
        operation_hash: Option<String>,
    ) -> Result<(), diesel::result::Error> {
        let _result = diesel::update(operation_requests::table.find(id))
            .set((
                operation_requests::dsl::state.eq::<i16>(OperationRequestState::Injected.into()),
                operation_requests::dsl::operation_hash.eq(operation_hash),
            ))
            .execute(conn)?;

        Ok(())
    }

    pub fn max_nonce(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        contract_id: &Uuid,
    ) -> Result<i64, diesel::result::Error> {
        let op: OperationRequest = operation_requests::table
            .filter(operation_requests::dsl::contract_id.eq(contract_id))
            .order_by(operation_requests::dsl::nonce.desc())
            .first(conn)?;

        Ok(op.nonce as i64)
    }

    pub fn operation_approvals(
        &self,
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
    ) -> Result<Vec<(OperationApproval, User)>, diesel::result::Error> {
        OperationApproval::belonging_to(self)
            .inner_join(users::table)
            .load::<(OperationApproval, User)>(conn)
    }

    pub fn delete_and_fix_next_nonces(
        &self,
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
    ) -> Result<(), diesel::result::Error> {
        conn.transaction::<_, diesel::result::Error, _>(|| {
            Self::delete(conn, &self.id)?;
            let updated_operation_requests: Vec<OperationRequest> = diesel::update(
                operation_requests::table.filter(operation_requests::dsl::nonce.gt(self.nonce)),
            )
            .set(operation_requests::dsl::nonce.eq(operation_requests::dsl::nonce - 1))
            .get_results(conn)?;

            let _ = diesel::delete(OperationApproval::belonging_to(&updated_operation_requests))
                .execute(conn)?;
            Ok(())
        })?;

        Ok(())
    }

    pub fn insert(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        new_operation_request: &NewOperationRequest,
    ) -> Result<OperationRequest, diesel::result::Error> {
        diesel::insert_into(operation_requests::table)
            .values(new_operation_request)
            .get_result(conn)
    }

    pub fn get_list(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        kind: OperationRequestKind,
        contract_id: Uuid,
        state: Option<OperationRequestState>,
        page: i64,
        limit: i64,
    ) -> Result<(Vec<(OperationRequest, User)>, i64), diesel::result::Error> {
        let mut query = operation_requests::table
            .filter(operation_requests::dsl::kind.eq::<i16>(kind.into()))
            .filter(operation_requests::dsl::contract_id.eq(contract_id))
            .order_by(operation_requests::dsl::created_at)
            .inner_join(users::table)
            .into_boxed();

        if let Some(state) = state {
            query = query.filter(operation_requests::dsl::state.eq::<i16>(state.into()));
        }

        let query = query.paginate(page).per_page(limit);

        query.load_and_count_pages::<(OperationRequest, User)>(&conn)
    }

    pub fn delete(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: &Uuid,
    ) -> Result<(), diesel::result::Error> {
        diesel::delete(operation_requests::table.find(id)).execute(conn)?;
        Ok(())
    }
}

#[derive(Insertable, Debug)]
#[table_name = "operation_requests"]
pub struct NewOperationRequest {
    pub gatekeeper_id: Uuid,
    pub contract_id: Uuid,
    pub target_address: Option<String>,
    pub amount: BigDecimal,
    pub kind: i16,
    pub chain_id: String,
    pub nonce: i64,
}

#[derive(AsChangeset, Debug)]
#[table_name = "operation_requests"]
pub struct UpdateOperation {
    pub id: Uuid,
    pub operation_hash: String,
    pub state: i16,
}
