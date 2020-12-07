use uuid::Uuid;
use chrono::NaiveDateTime;

use crate::db::schema::*;
use crate::db::models::{ contract::Contract, gatekeeper::Gatekeeper };


#[derive(Queryable, Identifiable, Associations, Debug)]
#[belongs_to(Gatekeeper, foreign_key = "requester")]
#[belongs_to(Contract, foreign_key = "destination")]
pub struct OperationRequest {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub requester: Uuid,
    pub destination: Uuid,
    pub target_address: String,
    pub amount: i64,
    pub kind: i16,
    pub gk_signature: String,
    pub chain_id: String,
    pub nonce: i32,
    pub state: i16
}
