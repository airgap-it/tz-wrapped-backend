use chrono::NaiveDateTime;
use diesel::{prelude::*, r2d2::ConnectionManager, r2d2::PooledConnection};
use uuid::Uuid;

use crate::db::models::{operation_request::OperationRequest, user::User};
use crate::db::schema::*;

#[derive(Queryable, Identifiable, Associations, Debug, Clone)]
#[belongs_to(User, foreign_key = "user_id")]
#[belongs_to(OperationRequest, foreign_key = "operation_request_id")]
pub struct ProposedUser {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub user_id: Uuid,
    pub operation_request_id: Uuid,
}

impl ProposedUser {
    pub fn get(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: Uuid,
    ) -> Result<ProposedUser, diesel::result::Error> {
        let result: ProposedUser = proposed_users::table.find(id).first(conn)?;

        Ok(result)
    }

    pub fn insert(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        operation_request: &OperationRequest,
        users: &Vec<User>,
    ) -> Result<usize, diesel::result::Error> {
        let new_proposed_users: Vec<NewProposedUser> = users
            .iter()
            .map(|user| NewProposedUser {
                user_id: user.id,
                operation_request_id: operation_request.id,
            })
            .collect();

        diesel::insert_into(proposed_users::table)
            .values(new_proposed_users)
            .execute(conn)
    }
}

#[derive(Insertable, Debug)]
#[table_name = "proposed_users"]
pub struct NewProposedUser {
    pub user_id: Uuid,
    pub operation_request_id: Uuid,
}
