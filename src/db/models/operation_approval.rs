use crate::db::schema::*;
use crate::{
    api::models::user::UserState,
    db::models::{operation_request::OperationRequest, user::User},
};
use chrono::NaiveDateTime;
use diesel::{prelude::*, r2d2::ConnectionManager, PgConnection};
use r2d2::PooledConnection;
use uuid::Uuid;

use super::pagination::Paginate;

#[derive(Queryable, Identifiable, Associations, Debug)]
#[belongs_to(User, foreign_key = "keyholder_id")]
#[belongs_to(OperationRequest, foreign_key = "operation_request_id")]
pub struct OperationApproval {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub keyholder_id: Uuid,
    pub operation_request_id: Uuid,
    pub signature: String,
}

impl OperationApproval {
    pub fn count(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        operation_request_id: &Uuid,
    ) -> Result<i64, diesel::result::Error> {
        let count = operation_approvals::dsl::operation_approvals
            .filter(operation_approvals::dsl::operation_request_id.eq(operation_request_id))
            .inner_join(users::table)
            .filter(users::dsl::state.eq::<i16>(UserState::Active.into()))
            .count()
            .get_result::<i64>(conn)?;

        Ok(count)
    }

    pub fn get(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: Uuid,
    ) -> Result<OperationApproval, diesel::result::Error> {
        let result: OperationApproval = operation_approvals::dsl::operation_approvals
            .find(id)
            .first(conn)?;

        Ok(result)
    }

    pub fn get_list(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        operation_request_id: Uuid,
        page: i64,
        limit: i64,
    ) -> Result<(Vec<(OperationApproval, User)>, i64), diesel::result::Error> {
        let approvals_query = operation_approvals::dsl::operation_approvals
            .filter(operation_approvals::operation_request_id.eq(operation_request_id))
            .order_by(operation_approvals::dsl::created_at)
            .inner_join(users::table)
            .paginate(page)
            .per_page(limit);

        approvals_query.load_and_count_pages::<(OperationApproval, User)>(conn)
    }

    pub fn insert(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        new_operation_approval: NewOperationApproval,
    ) -> Result<OperationApproval, diesel::result::Error> {
        diesel::insert_into(operation_approvals::dsl::operation_approvals)
            .values(new_operation_approval)
            .get_result(conn)
    }
}

#[derive(Insertable)]
#[table_name = "operation_approvals"]
pub struct NewOperationApproval {
    pub keyholder_id: Uuid,
    pub operation_request_id: Uuid,
    pub signature: String,
}
