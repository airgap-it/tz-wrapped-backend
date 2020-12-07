use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ListResponse<T> {
    pub page: i64,
    pub total_pages: i64,
    pub results: Vec<T>
}
