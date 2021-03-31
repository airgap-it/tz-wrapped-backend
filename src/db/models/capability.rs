use super::contract::Contract;
use crate::db::schema::capabilities;
use chrono::NaiveDateTime;
use diesel::{prelude::*, r2d2::ConnectionManager, PgConnection};
use r2d2::PooledConnection;
use uuid::Uuid;

#[derive(Queryable, Identifiable, Associations, Clone, Debug)]
#[table_name = "capabilities"]
#[belongs_to(Contract, foreign_key = "contract_id")]
pub struct Capability {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub contract_id: Uuid,
    pub operation_request_kind: i16,
}

impl Capability {
    pub fn insert(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        new_capabilities: Vec<NewCapability>,
    ) -> Result<Vec<Capability>, diesel::result::Error> {
        let capabilities = diesel::insert_into(capabilities::table)
            .values(&new_capabilities)
            .get_results(conn)?;

        Ok(capabilities)
    }

    pub fn delete(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        to_remove: Vec<Uuid>,
    ) -> Result<(), diesel::result::Error> {
        diesel::delete(capabilities::table.filter(capabilities::dsl::id.eq_any(to_remove)))
            .execute(conn)?;
        Ok(())
    }
}

#[derive(Insertable)]
#[table_name = "capabilities"]
pub struct NewCapability {
    pub contract_id: Uuid,
    pub operation_request_kind: i16,
}
