use uuid::Uuid;
use chrono::NaiveDateTime;
use crate::db::schema::*;
use crate::db::models::{ operation_request::OperationRequest, keyholder::Keyholder };

#[derive(Queryable, Identifiable, Associations, Debug)]
#[belongs_to(Keyholder, foreign_key = "approver")]
#[belongs_to(OperationRequest, foreign_key = "request")]
pub struct OperationApproval {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub approver: Uuid,
    pub request: Uuid,
    pub kh_signature: String
}
