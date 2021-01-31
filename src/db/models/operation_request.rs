use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use diesel::{prelude::*, r2d2::ConnectionManager, r2d2::PooledConnection};
use uuid::Uuid;

use crate::{
    api::models::operation_request::OperationRequestKind,
    db::schema::{contracts, operation_requests, users},
};
use crate::{
    api::models::operation_request::OperationRequestState,
    db::models::{contract::Contract, operation_approval::OperationApproval, user::User},
};

use super::pagination::Paginate;

#[derive(Queryable, Identifiable, Associations, Debug)]
#[belongs_to(User, foreign_key = "gatekeeper_id")]
#[belongs_to(Contract, foreign_key = "contract_id")]
pub struct OperationRequest {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub gatekeeper_id: Uuid,
    pub contract_id: Uuid,
    pub target_address: Option<String>,
    pub amount: BigDecimal,
    pub kind: i16,
    pub chain_id: String,
    pub nonce: i64,
    pub state: i16,
    pub operation_hash: Option<String>,
}

impl OperationRequest {
    pub fn get(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: &Uuid,
    ) -> Result<OperationRequest, diesel::result::Error> {
        operation_requests::table.find(id).first(conn)
    }

    pub fn get_with_operation_approvals(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: &Uuid,
    ) -> Result<(OperationRequest, Vec<(OperationApproval, User)>), diesel::result::Error> {
        let operation_request: OperationRequest = operation_requests::table.find(id).first(conn)?;
        let operation_approvals = operation_request.operation_approvals(conn)?;
        Ok((operation_request, operation_approvals))
    }

    pub fn get_with_contract(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: &Uuid,
    ) -> Result<(OperationRequest, Contract), diesel::result::Error> {
        operation_requests::table
            .find(id)
            .inner_join(contracts::table)
            .first(conn)
    }

    pub fn mark_approved(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: &Uuid,
    ) -> Result<(), diesel::result::Error> {
        let _result = diesel::update(operation_requests::table.find(id))
            .set(operation_requests::dsl::state.eq::<i16>(OperationRequestState::Approved.into()))
            .execute(conn)?;

        Ok(())
    }

    pub fn mark_injected(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: &Uuid,
        operation_hash: Option<String>,
    ) -> Result<(), diesel::result::Error> {
        let _result = diesel::update(operation_requests::table.find(id))
            .set((
                operation_requests::dsl::state.eq::<i16>(OperationRequestState::Injected.into()),
                operation_requests::dsl::operation_hash.eq(operation_hash),
            ))
            .execute(conn)?;

        Ok(())
    }

    pub fn max_nonce(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        contract_id: &Uuid,
    ) -> Result<i64, diesel::result::Error> {
        let op: OperationRequest = operation_requests::table
            .filter(operation_requests::dsl::contract_id.eq(contract_id))
            .order_by(operation_requests::dsl::nonce.desc())
            .first(conn)?;

        Ok(op.nonce as i64)
    }

    pub fn operation_approvals(
        &self,
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
    ) -> Result<Vec<(OperationApproval, User)>, diesel::result::Error> {
        OperationApproval::belonging_to(self)
            .inner_join(users::table)
            .load::<(OperationApproval, User)>(conn)
    }

    pub fn delete_and_fix_next_nonces(
        &self,
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
    ) -> Result<(), diesel::result::Error> {
        conn.transaction::<_, diesel::result::Error, _>(|| {
            Self::delete(conn, &self.id)?;

            let updated_operation_requests: Vec<OperationRequest> = diesel::update(
                operation_requests::table
                    .filter(operation_requests::dsl::nonce.gt(self.nonce))
                    .filter(operation_requests::dsl::contract_id.eq(self.contract_id))
                    .filter(
                        operation_requests::dsl::state
                            .ne::<i16>(OperationRequestState::Injected.into()),
                    ),
            )
            .set((
                operation_requests::dsl::nonce.eq(operation_requests::dsl::nonce - 1),
                operation_requests::dsl::state.eq::<i16>(OperationRequestState::Open.into()),
            ))
            .get_results(conn)?;

            if !updated_operation_requests.is_empty() {
                let _ =
                    diesel::delete(OperationApproval::belonging_to(&updated_operation_requests))
                        .execute(conn)?;
            }

            Ok(())
        })?;

        Ok(())
    }

    pub fn insert(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        new_operation_request: &NewOperationRequest,
    ) -> Result<OperationRequest, diesel::result::Error> {
        diesel::insert_into(operation_requests::table)
            .values(new_operation_request)
            .get_result(conn)
    }

    pub fn get_list(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        kind: OperationRequestKind,
        contract_id: Uuid,
        state: Option<OperationRequestState>,
        page: i64,
        limit: i64,
    ) -> Result<
        (
            Vec<(OperationRequest, User, Vec<(OperationApproval, User)>)>,
            i64,
        ),
        diesel::result::Error,
    > {
        let mut query = operation_requests::table
            .filter(operation_requests::dsl::kind.eq::<i16>(kind.into()))
            .filter(operation_requests::dsl::contract_id.eq(contract_id))
            .order_by(operation_requests::dsl::created_at)
            .inner_join(users::table)
            .into_boxed();

        if let Some(state) = state {
            query = query.filter(operation_requests::dsl::state.eq::<i16>(state.into()));
        }

        let query = query.paginate(page).per_page(limit);

        let (result, page_count) = query.load_and_count_pages::<(OperationRequest, User)>(&conn)?;

        let (operation_requests, users): (Vec<OperationRequest>, Vec<User>) =
            result.into_iter().unzip();

        let operation_approvals: Vec<OperationApproval> =
            OperationApproval::belonging_to(&operation_requests).load(conn)?;
        let keyholders: Vec<User> = User::get_all_with_ids(
            conn,
            operation_approvals
                .iter()
                .map(|operation_approval| &operation_approval.keyholder_id)
                .collect(),
        )?;
        let operation_approvals_and_keyholders: Vec<(OperationApproval, User)> =
            operation_approvals
                .into_iter()
                .map(|operation_approval| {
                    let keyholder = keyholders
                        .iter()
                        .find(|keyholder| keyholder.id == operation_approval.keyholder_id)
                        .unwrap();

                    (operation_approval, keyholder.clone())
                })
                .collect();
        let grouped_operation_approvals: Vec<Vec<(OperationApproval, User)>> =
            operation_approvals_and_keyholders.grouped_by(&operation_requests);

        let operation_requests_and_users: Vec<(OperationRequest, User)> =
            operation_requests.into_iter().zip(users).collect();
        let result: Vec<(OperationRequest, User, Vec<(OperationApproval, User)>)> =
            operation_requests_and_users
                .into_iter()
                .zip(grouped_operation_approvals)
                .map(|((operation_request, user), operation_approvals)| {
                    (operation_request, user, operation_approvals)
                })
                .collect();

        Ok((result, page_count))
    }

    pub fn delete(
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        id: &Uuid,
    ) -> Result<(), diesel::result::Error> {
        diesel::delete(operation_requests::table.find(id)).execute(conn)?;
        Ok(())
    }
}

#[derive(Insertable, Debug)]
#[table_name = "operation_requests"]
pub struct NewOperationRequest {
    pub gatekeeper_id: Uuid,
    pub contract_id: Uuid,
    pub target_address: Option<String>,
    pub amount: BigDecimal,
    pub kind: i16,
    pub chain_id: String,
    pub nonce: i64,
}

#[derive(AsChangeset, Debug)]
#[table_name = "operation_requests"]
pub struct UpdateOperation {
    pub id: Uuid,
    pub operation_hash: String,
    pub state: i16,
}
