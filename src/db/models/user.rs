use chrono::NaiveDateTime;
use diesel::{prelude::*, r2d2::ConnectionManager, r2d2::PooledConnection};
use uuid::Uuid;

use crate::api::models::{
    error::APIError,
    user::{UserKind, UserState},
};
use crate::crypto;
use crate::db::schema::*;
use crate::tezos;

use super::pagination::Paginate;

#[derive(Queryable, Identifiable, Debug)]
pub struct User {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub public_key: String,
    pub address: String,
    pub contract_id: Uuid,
    pub kind: i16,
    pub state: i16,
    pub display_name: String,
    pub email: Option<String>,
}

impl User {
    pub fn verify_message(&self, message: &[u8], signature: &str) -> Result<bool, APIError> {
        let signature_bytes = tezos::edsig_to_bytes(signature)?;
        let pk = tezos::edpk_to_bytes(&self.public_key)?;
        let is_match = crypto::verify_detached(message, signature_bytes, pk);

        Ok(is_match)
    }

    pub fn get(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: Uuid,
    ) -> Result<User, diesel::result::Error> {
        let result: User = users::dsl::users.find(id).first(conn)?;

        Ok(result)
    }

    pub fn get_active(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        address: &str,
        kind: UserKind,
        contract_id: Uuid,
    ) -> Result<User, diesel::result::Error> {
        let result: User = users::dsl::users
            .filter(users::dsl::address.eq(address))
            .filter(users::dsl::contract_id.eq(contract_id))
            .filter(users::dsl::kind.eq::<i16>(kind.into()))
            .filter(users::dsl::state.eq::<i16>(UserState::Active.into()))
            .first(conn)?;

        Ok(result)
    }

    pub fn get_all(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        kind: Option<UserKind>,
        contract_id: Option<Uuid>,
        state: Option<UserState>,
        address: Option<&String>,
    ) -> Result<Vec<User>, diesel::result::Error> {
        let mut query = users::dsl::users
            .order_by(users::dsl::created_at)
            .into_boxed();

        if let Some(kind) = kind {
            query = query.filter(users::dsl::kind.eq::<i16>(kind.into()));
        }

        if let Some(contract_id) = contract_id {
            query = query.filter(users::dsl::contract_id.eq(contract_id));
        }

        if let Some(state) = state {
            query = query.filter(users::dsl::state.eq::<i16>(state.into()));
        }

        if let Some(address) = address {
            query = query.filter(users::dsl::address.eq(address));
        }

        let result = query.load(conn)?;

        Ok(result)
    }

    pub fn get_all_active(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        contract_id: Uuid,
        kind: UserKind,
    ) -> Result<Vec<User>, diesel::result::Error> {
        User::get_all(
            &conn,
            Some(kind),
            Some(contract_id),
            Some(UserState::Active),
            None,
        )
    }

    pub fn get_list(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        state: Option<UserState>,
        kind: Option<UserKind>,
        contract_id: Option<Uuid>,
        address: Option<&String>,
        page: i64,
        limit: i64,
    ) -> Result<(Vec<User>, i64), diesel::result::Error> {
        let mut users_query = users::dsl::users
            .filter(users::dsl::state.eq::<i16>(state.unwrap_or(UserState::Active).into()))
            .order_by(users::dsl::created_at)
            .into_boxed();

        if let Some(kind) = kind {
            users_query = users_query.filter(users::dsl::kind.eq::<i16>(kind.into()));
        }

        if let Some(contract_id) = contract_id {
            users_query = users_query.filter(users::dsl::contract_id.eq(contract_id));
        }

        if let Some(address) = address {
            users_query = users_query.filter(users::dsl::address.eq(address));
        }

        let paginated_query = users_query.paginate(page).per_page(limit);

        paginated_query.load_and_count_pages::<User>(&conn)
    }

    // TODO: refactor and optimize this method
    pub fn sync_users(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        contract_id: Uuid,
        kind: UserKind,
        users: &Vec<SyncUser>,
    ) -> Result<usize, APIError> {
        let stored_users = User::get_all(conn, Some(kind), Some(contract_id), None, None)?;

        let to_deactivate: Vec<_> = stored_users
            .iter()
            .filter(|stored_user| {
                let found = users
                    .iter()
                    .find(|user| user.public_key == stored_user.public_key);
                if let Some(_user) = found {
                    false
                } else {
                    true
                }
            })
            .map(|user| &user.id)
            .collect();

        let to_add: Vec<_> = users
            .iter()
            .filter(|user| {
                let found = stored_users
                    .iter()
                    .find(|stored_user| user.public_key == stored_user.public_key);

                if let None = found {
                    true
                } else {
                    false
                }
            })
            .map(|user| {
                Ok(NewUser {
                    public_key: user.public_key.clone(),
                    address: tezos::edpk_to_tz1(&user.public_key)?,
                    contract_id,
                    kind: kind.into(),
                    display_name: user.display_name.clone(),
                    email: user.email.clone(),
                })
            })
            .collect::<Result<Vec<NewUser>, APIError>>()?;

        let to_update: Vec<_> = users
            .iter()
            .filter_map(|user| {
                let found = stored_users
                    .iter()
                    .find(|stored_user| stored_user.public_key == user.public_key);

                if let Some(stored_user) = found {
                    let inactive: i16 = UserState::Inactive.into();
                    let has_changes = stored_user.display_name != user.display_name
                        || stored_user.email != user.email
                        || stored_user.state == inactive;
                    if has_changes {
                        Some(UpdateUser {
                            id: stored_user.id,
                            state: UserState::Active.into(),
                            display_name: user.display_name.clone(),
                            email: user.email.clone(),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        let mut changes: usize = 0;

        if !to_deactivate.is_empty() {
            let deactivated =
                diesel::update(users::dsl::users.filter(users::dsl::id.eq_any(to_deactivate)))
                    .set(users::dsl::state.eq::<i16>(UserState::Inactive.into()))
                    .execute(conn)?;

            changes += deactivated;
        }

        if !to_add.is_empty() {
            let added = diesel::insert_into(users::dsl::users)
                .values(to_add)
                .execute(conn)?;

            changes += added
        }

        if !to_update.is_empty() {
            for update in to_update {
                changes += diesel::update(users::dsl::users.find(update.id))
                    .set(update)
                    .execute(conn)?;
            }
        }

        Ok(changes)
    }
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser {
    pub public_key: String,
    pub address: String,
    pub contract_id: Uuid,
    pub kind: i16,
    pub display_name: String,
    pub email: Option<String>,
}

#[derive(AsChangeset, Debug)]
#[table_name = "users"]
pub struct UpdateUser {
    pub id: Uuid,
    pub state: i16,
    pub display_name: String,
    pub email: Option<String>,
}

pub struct SyncUser {
    pub public_key: String,
    pub display_name: String,
    pub email: Option<String>,
}
