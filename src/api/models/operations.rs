use diesel::{r2d2::ConnectionManager, PgConnection};
use r2d2::PooledConnection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::NaiveDateTime;

use crate::db::models::{contract::Contract, gatekeeper::Gatekeeper, operation_request::OperationRequest };
use crate::db::schema::operation_requests;

#[derive(Serialize, Deserialize)]
pub struct OperationResponse {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub requester: Gatekeeper,
    pub destination: Contract,
    pub target_address: String,
    pub amount: i64,
    pub kind: OperationKind,
    pub gk_signature: String,
    pub chain_id: String,
    pub nonce: i32,
    pub state: OperationState
}

impl OperationResponse {

    pub fn from(operation: OperationRequest, gatekeeper: Gatekeeper, contract: Contract) -> OperationResponse {
        OperationResponse {
            id: operation.id,
            created_at: operation.created_at,
            updated_at: operation.updated_at,
            requester: gatekeeper,
            destination: contract,
            target_address: operation.target_address,
            amount: operation.amount,
            kind: match operation.kind {
                0 => OperationKind::Mint,
                1 => OperationKind::Burn,
                _ => OperationKind::Mint
            },
            gk_signature: operation.gk_signature,
            chain_id: operation.chain_id,
            nonce: operation.nonce,
            state: match operation.state {
                0 => OperationState::Open,
                1 => OperationState::Approved,
                2 => OperationState::Submitted,
                _ => OperationState::Open
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct OperationBody {
    pub destination: Uuid,
    pub target_address: String,
    pub amount: i64,
    pub kind: OperationKind,
    pub gk_signature: String,
    pub chain_id: String,
    pub nonce: i32
}

impl OperationBody {

    pub fn find_and_validate_gatekeeper(&self, conn: &PooledConnection<ConnectionManager<PgConnection>>) -> Result<Gatekeeper, diesel::result::Error> {
        todo!()
    }
}

#[derive(Insertable)]
#[table_name = "operation_requests"]
pub struct NewOperation {
    pub requester: Uuid,
    pub destination: Uuid,
    pub target_address: String,
    pub amount: i64,
    pub kind: i16,
    pub gk_signature: String,
    pub chain_id: String,
    pub nonce: i32
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum OperationKind {
    Mint = 0,
    Burn = 1
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum OperationState {
    Open = 0,
    Approved = 1,
    Submitted = 2
}
