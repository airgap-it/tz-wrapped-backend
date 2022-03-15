use crate::db::schema::node_endpoints;
use crate::settings::TezosNode;
use crate::Conn;
use chrono::NaiveDateTime;
use diesel::dsl::any;
use diesel::prelude::*;
use uuid::Uuid;

use super::pagination::Paginate;

#[derive(Queryable, Identifiable, Clone, Debug)]
#[table_name = "node_endpoints"]
pub struct NodeEndpoint {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub name: String,
    pub url: String,
    pub network: String,
    pub selected: bool,
}

impl NodeEndpoint {
    pub fn insert(
        conn: &Conn,
        new_node_endpoint: Vec<NewNodeEndpoint>,
    ) -> Result<Vec<NodeEndpoint>, diesel::result::Error> {
        let node_endpoint = diesel::insert_into(node_endpoints::table)
            .values(&new_node_endpoint)
            .get_results(conn)?;

        Ok(node_endpoint)
    }

    pub fn delete(conn: &Conn, to_remove: Vec<Uuid>) -> Result<(), diesel::result::Error> {
        diesel::delete(node_endpoints::table.filter(node_endpoints::dsl::id.eq_any(to_remove)))
            .execute(conn)?;
        Ok(())
    }

    pub fn get_selected(conn: &Conn) -> Result<NodeEndpoint, diesel::result::Error> {
        let result = node_endpoints::table
            .filter(node_endpoints::dsl::selected.eq(true))
            .first(conn)?;

        Ok(result)
    }

    pub fn set_selected(conn: &Conn, uuid: Uuid) -> Result<(), diesel::result::Error> {
        let selected = Self::get_selected(conn)?;
        if selected.id == uuid {
            return Ok(());
        }

        conn.transaction::<_, diesel::result::Error, _>(|| {
            diesel::update(node_endpoints::table.find(selected.id))
                .set(node_endpoints::dsl::selected.eq(false))
                .execute(conn)?;
            diesel::update(node_endpoints::table.find(uuid))
                .set(node_endpoints::dsl::selected.eq(true))
                .execute(conn)?;

            Ok(())
        })
    }

    pub fn get_list(
        conn: &Conn,
        page: i64,
        limit: i64,
    ) -> Result<(Vec<NodeEndpoint>, i64), diesel::result::Error> {
        let query = node_endpoints::table
            .order_by(node_endpoints::dsl::name.asc())
            .paginate(page)
            .per_page(limit);

        query.load_and_count_pages::<NodeEndpoint>(conn)
    }

    pub fn get_all(conn: &Conn) -> Result<Vec<NodeEndpoint>, diesel::result::Error> {
        node_endpoints::table.load(conn)
    }

    pub fn sync(conn: &Conn, tezos_nodes: &Vec<TezosNode>) -> Result<usize, diesel::result::Error> {
        let stored_endpoints = NodeEndpoint::get_all(conn)?;

        let to_remove: Vec<_> = stored_endpoints
            .iter()
            .filter(|stored_endpoint| {
                tezos_nodes
                    .iter()
                    .find(|tezos_node| tezos_node.url == stored_endpoint.url)
                    .is_none()
            })
            .map(|endpoint| endpoint.id)
            .collect();

        let to_add: Vec<_> = tezos_nodes
            .iter()
            .filter(|tezos_node| {
                stored_endpoints
                    .iter()
                    .find(|stored_endpoint| tezos_node.url == stored_endpoint.url)
                    .is_none()
            })
            .map(|tezos_node| NewNodeEndpoint {
                name: tezos_node.name.clone(),
                url: tezos_node.url.clone(),
                network: tezos_node.network.clone(),
                selected: false,
            })
            .collect();

        let to_update: Vec<_> = tezos_nodes
            .iter()
            .filter_map(|tezos_node| {
                let found = stored_endpoints.iter().find(|stored_endpoint| {
                    stored_endpoint.url == tezos_node.url
                        && (stored_endpoint.name != tezos_node.name
                            || stored_endpoint.network != tezos_node.network)
                });

                found.map(|stored_endpoint| UpdateNodeEndpoint {
                    id: stored_endpoint.id,
                    name: Some(tezos_node.name.clone()),
                    network: Some(tezos_node.network.clone()),
                    selected: None,
                })
            })
            .collect();

        let mut changes: usize = 0;

        if !to_remove.is_empty() {
            let removed = diesel::delete(
                node_endpoints::table.filter(node_endpoints::dsl::id.eq(any(to_remove))),
            )
            .execute(conn)?;

            changes += removed;
        }

        if !to_add.is_empty() {
            let added = diesel::insert_into(node_endpoints::table)
                .values(to_add)
                .execute(conn)?;

            changes += added;
        }

        if !to_update.is_empty() {
            for update in to_update {
                changes += diesel::update(node_endpoints::table.find(update.id))
                    .set(update)
                    .execute(conn)?;
            }
        }

        if NodeEndpoint::get_selected(conn).is_err() {
            let first: NodeEndpoint = node_endpoints::table
                .filter(node_endpoints::dsl::name.eq("Papers"))
                .order_by(node_endpoints::dsl::created_at.asc())
                .first(conn)
                .or_else(|_| {
                    node_endpoints::table
                        .order_by(node_endpoints::dsl::created_at.asc())
                        .first(conn)
                })?;
            diesel::update(node_endpoints::table.find(first.id))
                .set(UpdateNodeEndpoint {
                    id: first.id,
                    name: None,
                    network: None,
                    selected: Some(true),
                })
                .execute(conn)?;
        }

        Ok(changes)
    }
}

#[derive(Insertable)]
#[table_name = "node_endpoints"]
pub struct NewNodeEndpoint {
    pub name: String,
    pub url: String,
    pub network: String,
    pub selected: bool,
}

#[derive(AsChangeset, Identifiable, Debug)]
#[table_name = "node_endpoints"]
pub struct UpdateNodeEndpoint {
    pub id: Uuid,
    pub name: Option<String>,
    pub network: Option<String>,
    pub selected: Option<bool>,
}
