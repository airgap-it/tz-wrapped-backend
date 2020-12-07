use uuid::Uuid;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::db::schema::*;

#[derive(Queryable, Identifiable, Debug, Serialize, Deserialize)]
pub struct Gatekeeper {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub public_key: String,
    pub contract_id: Uuid
}