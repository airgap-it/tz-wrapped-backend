use chrono::NaiveDateTime;
use diesel::{prelude::*, r2d2::ConnectionManager, r2d2::PooledConnection};
use uuid::Uuid;

use crate::{api::models::authentication::AuthenticationChallengeState, db::schema::*};

#[derive(Queryable, Identifiable, Debug)]
pub struct AuthenticationChallenge {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
    pub address: String,
    pub challenge: String,
    pub state: i16,
}

impl AuthenticationChallenge {
    pub fn get(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: &Uuid,
    ) -> Result<AuthenticationChallenge, diesel::result::Error> {
        let result: AuthenticationChallenge =
            authentication_challenges::dsl::authentication_challenges
                .find(id)
                .filter(authentication_challenges::dsl::expires_at.gt(diesel::dsl::now))
                .first(conn)?;

        Ok(result)
    }

    pub fn mark_completed(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: &Uuid,
    ) -> Result<(), diesel::result::Error> {
        let _result =
            diesel::update(authentication_challenges::dsl::authentication_challenges.find(id))
                .set(
                    authentication_challenges::dsl::state
                        .eq::<i16>(AuthenticationChallengeState::Completed.into()),
                )
                .execute(conn)?;

        Ok(())
    }

    pub fn insert(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        new_authentication_challenge: &NewAuthenticationChallenge,
    ) -> Result<AuthenticationChallenge, diesel::result::Error> {
        diesel::insert_into(authentication_challenges::dsl::authentication_challenges)
            .values(new_authentication_challenge)
            .get_result(conn)
    }

    pub fn delete_expired(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
    ) -> Result<(), diesel::result::Error> {
        diesel::delete(
            authentication_challenges::dsl::authentication_challenges
                .filter(authentication_challenges::dsl::expires_at.lt(diesel::dsl::now)),
        )
        .execute(conn)?;

        Ok(())
    }
}

#[derive(Insertable, Debug)]
#[table_name = "authentication_challenges"]
pub struct NewAuthenticationChallenge {
    pub address: String,
    pub challenge: String,
}
