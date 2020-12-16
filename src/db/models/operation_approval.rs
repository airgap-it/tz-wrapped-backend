use crate::db::models::{operation_request::OperationRequest, user::User};
use crate::db::schema::*;
use chrono::NaiveDateTime;
use diesel::{prelude::*, r2d2::ConnectionManager, PgConnection};
use r2d2::PooledConnection;
use uuid::Uuid;

#[derive(Queryable, Identifiable, Associations, Debug)]
#[belongs_to(User, foreign_key = "approver")]
#[belongs_to(OperationRequest, foreign_key = "request")]
pub struct OperationApproval {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub approver: Uuid,
    pub request: Uuid,
    pub kh_signature: String,
}

impl OperationApproval {
    pub fn count(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        request_id: &Uuid,
    ) -> Result<i64, diesel::result::Error> {
        let count = operation_approvals::dsl::operation_approvals
            .filter(operation_approvals::dsl::request.eq(request_id))
            .count()
            .get_result::<i64>(conn)?;

        Ok(count)
    }

    pub fn get_by_id(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: Uuid,
    ) -> Result<OperationApproval, diesel::result::Error> {
        let result: OperationApproval = operation_approvals::dsl::operation_approvals
            .find(id)
            .first(conn)?;

        Ok(result)
    }
}

#[derive(Insertable)]
#[table_name = "operation_approvals"]
pub struct NewOperationApproval {
    pub approver: Uuid,
    pub request: Uuid,
    pub kh_signature: String,
}
