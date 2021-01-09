use chrono::NaiveDateTime;
use diesel::{prelude::*, r2d2::ConnectionManager, r2d2::PooledConnection};
use uuid::Uuid;

use crate::{api::models::operation_request::OperationRequestKind, db::schema::*};
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
    pub amount: i64,
    pub kind: i16,
    pub signature: String,
    pub chain_id: String,
    pub nonce: i64,
    pub state: i16,
    pub operation_hash: Option<String>,
}

impl OperationRequest {
    pub fn get_by_id(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: &Uuid,
    ) -> Result<OperationRequest, diesel::result::Error> {
        let result: OperationRequest = operation_requests::dsl::operation_requests
            .find(id)
            .first(conn)?;

        Ok(result)
    }

    pub fn get_by_id_with_contract(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: &Uuid,
    ) -> Result<(OperationRequest, Contract), diesel::result::Error> {
        operation_requests::dsl::operation_requests
            .find(id)
            .inner_join(contracts::table)
            .first(conn)
    }

    pub fn mark_approved(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: &Uuid,
    ) -> Result<(), diesel::result::Error> {
        let _result = diesel::update(operation_requests::dsl::operation_requests.find(id))
            .set(operation_requests::dsl::state.eq(OperationRequestState::Approved as i16))
            .execute(conn)?;

        Ok(())
    }

    pub fn mark_injected(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: &Uuid,
        operation_hash: String,
    ) -> Result<(), diesel::result::Error> {
        let _result = diesel::update(operation_requests::dsl::operation_requests.find(id))
            .set((
                operation_requests::dsl::state.eq(OperationRequestState::Injected as i16),
                operation_requests::dsl::operation_hash.eq(Some(operation_hash)),
            ))
            .execute(conn)?;

        Ok(())
    }

    pub fn max_nonce(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        contract_id: &Uuid,
    ) -> Result<i64, diesel::result::Error> {
        let op: OperationRequest = operation_requests::dsl::operation_requests
            .filter(operation_requests::dsl::contract_id.eq(contract_id))
            .order_by(operation_requests::dsl::nonce.desc())
            .first(conn)?;

        Ok(op.nonce as i64)
    }

    pub fn approvals(
        &self,
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
    ) -> Result<Vec<(OperationApproval, User)>, diesel::result::Error> {
        OperationApproval::belonging_to(self)
            .inner_join(users::dsl::users)
            .load::<(OperationApproval, User)>(conn)
    }

    pub fn insert(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        new_operation_request: &NewOperationRequest,
    ) -> Result<OperationRequest, diesel::result::Error> {
        diesel::insert_into(operation_requests::dsl::operation_requests)
            .values(new_operation_request)
            .get_result(conn)
    }

    pub fn get_list(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        kind: OperationRequestKind,
        contract_id: Uuid,
        page: i64,
        limit: i64,
    ) -> Result<(Vec<(OperationRequest, User)>, i64), diesel::result::Error> {
        let operations_query = operation_requests::dsl::operation_requests
            .filter(operation_requests::dsl::kind.eq(kind as i16))
            .filter(operation_requests::dsl::contract_id.eq(contract_id))
            .order_by(operation_requests::dsl::created_at)
            .inner_join(users::table)
            .paginate(page)
            .per_page(limit);

        operations_query.load_and_count_pages::<(OperationRequest, User)>(&conn)
    }
}

#[derive(Insertable, Debug)]
#[table_name = "operation_requests"]
pub struct NewOperationRequest {
    pub gatekeeper_id: Uuid,
    pub contract_id: Uuid,
    pub target_address: Option<String>,
    pub amount: i64,
    pub kind: i16,
    pub signature: String,
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
