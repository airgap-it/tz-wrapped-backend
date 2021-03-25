use std::convert::TryInto;

use actix_session::Session;
use actix_web::{
    web,
    web::{Path, Query},
    HttpResponse,
};
use diesel::{r2d2::ConnectionManager, r2d2::PooledConnection, PgConnection};
use serde::Deserialize;
use uuid::Uuid;

use crate::tezos::multisig;
use crate::tezos::multisig::Signature;
use crate::DbPool;
use crate::{
    api::models::user::UserKind,
    db::models::{
        contract::Contract, operation_request::OperationRequest as DBOperationRequest, user::User,
    },
};
use crate::{
    api::models::{
        common::{ListResponse, SignableMessageInfo},
        error::APIError,
        operation_request::OperationRequest,
        operation_request::{OperationRequestKind, OperationRequestState},
    },
    auth::get_current_user,
};
use crate::{auth::SessionUser, settings};

#[derive(Deserialize)]
pub struct Info {
    kind: OperationRequestKind,
    contract_id: Uuid,
    state: Option<OperationRequestState>,
    page: Option<i64>,
    limit: Option<i64>,
}

pub async fn operation_requests(
    pool: web::Data<DbPool>,
    query: Query<Info>,
    server_settings: web::Data<settings::Server>,
    session: Session,
) -> Result<HttpResponse, APIError> {
    let current_user = get_current_user(&session, server_settings.inactivity_timeout_seconds)?;

    let conn = pool.get()?;

    let page = query.page.unwrap_or(0);
    let limit = query.limit.unwrap_or(100);
    let kind = query.kind;
    let contract_id = query.contract_id;

    current_user.require_roles(vec![UserKind::Gatekeeper, UserKind::Keyholder], contract_id)?;

    let state = query.state;
    let result =
        web::block(move || load_operation_requests(&conn, page, limit, kind, contract_id, state))
            .await?;

    Ok(HttpResponse::Ok().json(result))
}

fn load_operation_requests(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    page: i64,
    limit: i64,
    kind: OperationRequestKind,
    contract_id: Uuid,
    state: Option<OperationRequestState>,
) -> Result<ListResponse<OperationRequest>, APIError> {
    let (operation_requests, total_pages) =
        DBOperationRequest::get_list(conn, kind, contract_id, state, page, limit)?;

    let results = operation_requests
        .into_iter()
        .map(
            |(operation_request, gatekeeper, operation_approvals, proposed_keyholders)| {
                OperationRequest::from(
                    operation_request,
                    gatekeeper,
                    operation_approvals,
                    proposed_keyholders,
                )
            },
        )
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ListResponse {
        page,
        total_pages,
        results,
    })
}

async fn load_operation_and_contract(
    pool: &web::Data<DbPool>,
    operation_request_id: &Uuid,
    current_user: SessionUser,
) -> Result<(DBOperationRequest, Contract, Option<Vec<User>>), APIError> {
    let conn = pool.get()?;
    let id = operation_request_id.clone();
    let result = web::block::<_, _, APIError>(move || {
        let operation_request = DBOperationRequest::get(&conn, &id)?;

        current_user.require_roles(
            vec![UserKind::Gatekeeper, UserKind::Keyholder],
            operation_request.contract_id,
        )?;

        let contract = Contract::get(&conn, &operation_request.contract_id)?;
        let proposed_keyholders = operation_request.proposed_keyholders(&conn)?;

        Ok((operation_request, contract, proposed_keyholders))
    })
    .await?;

    Ok(result)
}

#[derive(Deserialize)]
pub struct PathInfo {
    id: Uuid,
}

pub async fn operation_request(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
    server_settings: web::Data<settings::Server>,
    session: Session,
) -> Result<HttpResponse, APIError> {
    let current_user = get_current_user(&session, server_settings.inactivity_timeout_seconds)?;

    let conn = pool.get()?;
    let id = path.id;

    let (operation_request, user, operation_approvals, proposed_keyholders) =
        web::block::<_, _, APIError>(move || {
            let (operation_request, operation_approvals, proposed_keyholders) =
                DBOperationRequest::get_with_operation_approvals(&conn, &id)?;

            current_user.require_roles(
                vec![UserKind::Gatekeeper, UserKind::Keyholder],
                operation_request.contract_id,
            )?;

            let user = User::get(&conn, operation_request.user_id)?;

            Ok((
                operation_request,
                user,
                operation_approvals,
                proposed_keyholders,
            ))
        })
        .await?;

    Ok(HttpResponse::Ok().json(OperationRequest::from(
        operation_request,
        user,
        operation_approvals,
        proposed_keyholders,
    )?))
}

pub async fn signable_message(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
    tezos_settings: web::Data<settings::Tezos>,
    server_settings: web::Data<settings::Server>,
    session: Session,
) -> Result<HttpResponse, APIError> {
    let current_user = get_current_user(&session, server_settings.inactivity_timeout_seconds)?;

    let id = path.id;
    let (operation_request, contract, proposed_keyholders) =
        load_operation_and_contract(&pool, &id, current_user).await?;

    let multisig = multisig::get_multisig(
        contract.multisig_pkh.as_ref(),
        contract.kind.try_into()?,
        tezos_settings.node_url.as_ref(),
    );

    let signable_message = multisig
        .signable_message(&contract, &operation_request, proposed_keyholders)
        .await?;

    let signable_message_info: SignableMessageInfo = signable_message.try_into()?;

    Ok(HttpResponse::Ok().json(signable_message_info))
}

pub async fn operation_request_parameters(
    pool: web::Data<DbPool>,
    path: Path<PathInfo>,
    tezos_settings: web::Data<settings::Tezos>,
    server_settings: web::Data<settings::Server>,
    session: Session,
) -> Result<HttpResponse, APIError> {
    let current_user = get_current_user(&session, server_settings.inactivity_timeout_seconds)?;

    let conn = pool.get()?;
    let id = path.id;
    let (operation_request, contract, approvals, proposed_keyholders) =
        web::block::<_, _, APIError>(move || {
            let operation_request = DBOperationRequest::get(&conn, &id)?;

            current_user.require_roles(
                vec![UserKind::Gatekeeper, UserKind::Keyholder],
                operation_request.contract_id,
            )?;

            let contract = Contract::get(&conn, &operation_request.contract_id)?;
            let approvals = operation_request.operation_approvals(&conn)?;
            let proposed_keyholders = operation_request.proposed_keyholders(&conn)?;

            Ok((operation_request, contract, approvals, proposed_keyholders))
        })
        .await?;

    let mut multisig = multisig::get_multisig(
        contract.multisig_pkh.as_ref(),
        contract.kind.try_into()?,
        tezos_settings.node_url.as_ref(),
    );
    let signatures = approvals
        .iter()
        .map(|(approval, user)| Signature {
            value: approval.signature.as_ref(),
            public_key: user.public_key.as_ref(),
        })
        .collect::<Vec<Signature>>();
    let parameters = multisig
        .transaction_parameters(
            &contract,
            &operation_request,
            proposed_keyholders,
            signatures,
        )
        .await?;

    Ok(HttpResponse::Ok().json(parameters))
}
