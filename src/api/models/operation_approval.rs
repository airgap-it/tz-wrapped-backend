use std::convert::TryInto;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::models::{
    operation_approval::OperationApproval as DBOperationApproval, user::User as DBUser,
};

use super::{error::APIError, user::User};

#[derive(Serialize, Deserialize)]
pub struct OperationApproval {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub keyholder: User,
    pub operation_request_id: Uuid,
    pub signature: String,
}

impl OperationApproval {
    pub fn from(
        operation_approval: DBOperationApproval,
        keyholder: DBUser,
    ) -> Result<OperationApproval, APIError> {
        Ok(OperationApproval {
            id: operation_approval.id,
            created_at: operation_approval.created_at,
            updated_at: operation_approval.updated_at,
            keyholder: keyholder.try_into()?,
            operation_request_id: operation_approval.operation_request_id,
            signature: operation_approval.signature,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewOperationApproval {
    pub operation_request_id: Uuid,
    pub signature: String,
}
