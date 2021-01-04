use std::convert::TryInto;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::models::{operation_approval::OperationApproval, user::User};

use super::{error::APIError, users::UserResponse};

#[derive(Serialize, Deserialize)]
pub struct OperationApprovalResponse {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub approver: UserResponse,
    pub request_id: Uuid,
    pub kh_signature: String,
}

impl OperationApprovalResponse {
    pub fn from(
        operation: OperationApproval,
        keyholder: User,
    ) -> Result<OperationApprovalResponse, APIError> {
        Ok(OperationApprovalResponse {
            id: operation.id,
            created_at: operation.created_at,
            updated_at: operation.updated_at,
            approver: keyholder.try_into()?,
            request_id: operation.request,
            kh_signature: operation.kh_signature,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PostOperationApprovalBody {
    pub request: Uuid,
    pub kh_signature: String,
}
