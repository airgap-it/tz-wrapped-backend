use chrono::NaiveDateTime;
use diesel::{dsl::max, prelude::*, r2d2::ConnectionManager, r2d2::PooledConnection};
use uuid::Uuid;

use crate::db::schema::*;
use crate::{
    api::models::operations::OperationState,
    db::models::{contract::Contract, user::User},
};

use super::operation_approval::OperationApproval;

#[derive(Queryable, Identifiable, Associations, Debug)]
#[belongs_to(User, foreign_key = "requester")]
#[belongs_to(Contract, foreign_key = "destination")]
pub struct OperationRequest {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub requester: Uuid,
    pub destination: Uuid,
    pub target_address: Option<String>,
    pub amount: i64,
    pub kind: i16,
    pub gk_signature: String,
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

    pub fn mark_approved(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: &Uuid,
    ) -> Result<(), diesel::result::Error> {
        let _result = diesel::update(operation_requests::dsl::operation_requests.find(id))
            .set(operation_requests::dsl::state.eq(OperationState::Approved as i16))
            .execute(conn)?;

        Ok(())
    }

    pub fn max_nonce(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        contract_id: &Uuid,
    ) -> Result<i64, diesel::result::Error> {
        let nonce = operation_requests::dsl::operation_requests
            .select(max(operation_requests::dsl::nonce))
            .filter(operation_requests::dsl::destination.eq(contract_id))
            .execute(conn)?;

        Ok(nonce as i64)
    }

    pub fn approvals(
        &self,
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
    ) -> Result<Vec<(OperationApproval, User)>, diesel::result::Error> {
        OperationApproval::belonging_to(self)
            .inner_join(users::dsl::users)
            .load::<(OperationApproval, User)>(conn)
    }
}

#[derive(Insertable)]
#[table_name = "operation_requests"]
pub struct NewOperationRequest {
    pub requester: Uuid,
    pub destination: Uuid,
    pub target_address: Option<String>,
    pub amount: i64,
    pub kind: i16,
    pub gk_signature: String,
    pub chain_id: String,
    pub nonce: i64,
}
