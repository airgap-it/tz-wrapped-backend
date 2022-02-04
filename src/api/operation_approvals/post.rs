use std::convert::TryInto;

use actix_session::Session;
use actix_web::{web, HttpResponse};
use multisig::SignableMessage;
use uuid::Uuid;

use crate::db::models::{
    contract::Contract, operation_approval::NewOperationApproval as DBNewOperationApproval,
    operation_approval::OperationApproval as DBOperationApproval,
    operation_request::OperationRequest, user::User,
};
use crate::notifications::{notify_approval_received, notify_min_approvals_received};
use crate::settings;
use crate::tezos::multisig::{self, OperationRequestParams};
use crate::DbPool;
use crate::{api::models::user::UserKind, auth::get_current_user};
use crate::{
    api::models::{
        error::APIError,
        operation_approval::{NewOperationApproval, OperationApproval},
    },
    auth::SessionUser,
};

pub async fn operation_approval(
    pool: web::Data<DbPool>,
    tezos_settings: web::Data<settings::Tezos>,
    contract_settings: web::Data<Vec<settings::Contract>>,
    server_settings: web::Data<settings::Server>,
    body: web::Json<NewOperationApproval>,
    session: Session,
) -> Result<HttpResponse, APIError> {
    let current_user = get_current_user(&session, server_settings.inactivity_timeout_seconds)?;

    let (operation_request, contract, proposed_keyholders) =
        get_operation_request_and_contract(&pool, body.operation_request_id).await?;

    current_user.require_roles(vec![UserKind::Keyholder], contract.id)?;

    let mut multisig = multisig::get_multisig(
        contract.multisig_pkh.as_ref(),
        contract.kind.try_into()?,
        tezos_settings.node_url.as_ref(),
    );

    let operation_request_params = OperationRequestParams::from(operation_request.clone());
    let keyholder_public_keys = match proposed_keyholders {
        None => None,
        Some(keyholders) => Some(
            keyholders
                .into_iter()
                .map(|keyholder| keyholder.public_key)
                .collect(),
        ),
    };
    let signable_message = multisig
        .signable_message(&contract, &operation_request_params, keyholder_public_keys)
        .await?;

    let min_approvals = multisig.min_signatures().await?;

    crate::db::sync_keyholders(
        &pool,
        vec![contract.clone()],
        &tezos_settings.node_url,
        &contract_settings,
    )
    .await?;

    let keyholder =
        find_and_validate_keyholder(&pool, current_user, &signable_message, &contract, &body)
            .await?;
    let keyholder_id = keyholder.id;
    let inserted_approval = store_approval(&pool, keyholder_id, body.into_inner()).await?;

    let result = OperationApproval::from(inserted_approval, keyholder)?;

    let request_id = operation_request.id;
    let conn = pool.get()?;
    let total_approvals =
        web::block(move || DBOperationApproval::count(&conn, &request_id)).await?;

    let conn = pool.get()?;
    if total_approvals >= min_approvals {
        web::block::<_, _, APIError>(move || {
            OperationRequest::mark_approved(&conn, &request_id)?;

            let user = User::get(&conn, operation_request.user_id);
            let keyholders = User::get_all_active(&conn, contract.id, UserKind::Keyholder);
            if let Ok(user) = user {
                if let Ok(keyholders) = keyholders {
                    let _ = notify_min_approvals_received(
                        &user,
                        &keyholders,
                        &operation_request,
                        &contract,
                    );
                }
            }

            Ok(())
        })
        .await?;
    } else {
        let _ = web::block::<_, _, APIError>(move || {
            let user = User::get(&conn, operation_request.user_id)?;
            let keyholders = User::get_all_active(&conn, contract.id, UserKind::Keyholder)?;
            let approver = User::get(&conn, keyholder_id)?;
            let _ = notify_approval_received(
                &user,
                &approver,
                &keyholders,
                &operation_request,
                &contract,
            );

            Ok(())
        })
        .await;
    }

    Ok(HttpResponse::Ok().json(result))
}

async fn store_approval(
    pool: &web::Data<DbPool>,
    keyholder_id: Uuid,
    operation_approval: NewOperationApproval,
) -> Result<DBOperationApproval, APIError> {
    let conn = pool.get()?;
    Ok(web::block::<_, _, diesel::result::Error>(move || {
        let new_operation_approval = DBNewOperationApproval {
            keyholder_id,
            operation_request_id: operation_approval.operation_request_id,
            signature: operation_approval.signature,
        };

        DBOperationApproval::insert(&conn, new_operation_approval)
    })
    .await?)
}

async fn find_and_validate_keyholder(
    pool: &web::Data<DbPool>,
    current_user: SessionUser,
    message: &SignableMessage,
    contract: &Contract,
    operation_approval: &NewOperationApproval,
) -> Result<User, APIError> {
    let conn = pool.get()?;
    let contract_id = contract.id.clone();
    let keyholder = web::block::<_, _, APIError>(move || {
        Ok(User::get_active(
            &conn,
            &current_user.address,
            UserKind::Keyholder,
            contract_id,
        )?)
    })
    .await?;

    let hashed = message.blake2b_hash()?;
    let is_match = keyholder.verify_message(&hashed, &operation_approval.signature)?;
    if is_match {
        return Ok(keyholder);
    }
    Err(APIError::InvalidSignature)
}

async fn get_operation_request_and_contract(
    pool: &web::Data<DbPool>,
    operation_request_id: Uuid,
) -> Result<(OperationRequest, Contract, Option<Vec<User>>), APIError> {
    let conn = pool.get()?;

    let result: (OperationRequest, Contract, Option<Vec<User>>) =
        web::block::<_, _, APIError>(move || {
            let (operation_request, contract) =
                OperationRequest::get_with_contract(&conn, &operation_request_id)?;
            let proposed_keyholders = operation_request.proposed_keyholders(&conn)?;
            Ok((operation_request, contract, proposed_keyholders))
        })
        .await?;

    Ok(result)
}
