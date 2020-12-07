use uuid::Uuid;
use chrono::NaiveDateTime;
use crate::db::schema::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Identifiable, Clone, Serialize, Deserialize)]
pub struct Contract {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub pkh: String,
    pub token_id: i32,
    pub multisig_pkh: String,
    pub kind: i16,
    pub display_name: String
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ContractKind {
    FA1 = 0,
    FA2 = 1
}
