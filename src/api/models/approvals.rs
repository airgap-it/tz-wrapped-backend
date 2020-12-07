use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;
use uuid::Uuid;

use crate::db::models::{keyholder::Keyholder, operation_approval::OperationApproval};

#[derive(Serialize, Deserialize)]
pub struct ApprovalResponse {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub approver: Keyholder,
    pub request_id: Uuid,
    pub kh_signature: String
}


impl ApprovalResponse {

    pub fn from(operation: OperationApproval, keyholder: Keyholder) -> ApprovalResponse {
        ApprovalResponse {
            id: operation.id,
            created_at: operation.created_at,
            updated_at: operation.updated_at,
            approver: keyholder,
            request_id: operation.request,
            kh_signature: operation.kh_signature
        }
    }
}
