use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::models::node_endpoint::NodeEndpoint as DBNodeEndpoint;

#[derive(Debug, Serialize, Deserialize)]
pub struct TezosNode {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub name: String,
    pub url: String,
    pub network: String,
    pub selected: bool,
}

impl From<DBNodeEndpoint> for TezosNode {
    fn from(value: DBNodeEndpoint) -> Self {
        TezosNode {
            id: value.id,
            created_at: value.created_at,
            updated_at: value.updated_at,
            name: value.name,
            url: value.url,
            network: value.network,
            selected: value.selected,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SelectedTezosNode {
    pub id: Uuid,
}
