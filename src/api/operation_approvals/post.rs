use std::convert::TryInto;

use actix_session::Session;
use actix_web::{web, HttpResponse};
use log::info;
use multisig::SignableMessage;
use uuid::Uuid;

use crate::db::models::node_endpoint::NodeEndpoint;
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
    server_settings: web::Data<settings::Server>,
    body: web::Json<NewOperationApproval>,
    session: Session,
) -> Result<HttpResponse, APIError> {
    let current_user = get_current_user(&session, server_settings.inactivity_timeout_seconds)?;
    let new_operation_approval = body.into_inner();
    let (operation_request, contract, proposed_keyholders) =
        get_operation_request_and_contract(&pool, new_operation_approval.operation_request_id)
            .await?;

    current_user.require_roles(vec![UserKind::Keyholder], contract.id)?;

    info!("User {} submits new operation approval on contract {}:\n{:?}\nFor operation request:\n{:?}", current_user.address, contract.display_name, new_operation_approval, operation_request);

    let conn = pool.get()?;
    let node_url =
        web::block::<_, _, APIError>(move || Ok(NodeEndpoint::get_selected(&conn)?.url)).await?;
    let mut multisig = multisig::get_multisig(
        contract.multisig_pkh.as_ref(),
        contract.kind.try_into()?,
        &node_url,
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

    crate::db::sync_keyholders(&pool, vec![contract.clone()], &node_url).await?;

    let keyholder = find_keyholder_and_validate_signature(
        &pool,
        &signable_message,
        &contract,
        &new_operation_approval,
    )
    .await?;

    if keyholder.address != current_user.address {
        info!(
            "User {} is uploading signature for keyholder: {} / {}",
            current_user.address, keyholder.address, keyholder.public_key
        );
    }

    let keyholder_id = keyholder.id;
    let inserted_approval = store_approval(&pool, keyholder_id, new_operation_approval).await?;

    let result = OperationApproval::from(inserted_approval, keyholder)?;

    info!("Successfully created operation approval: {:?}", result);

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
        info!(
            "Enough signatures collected for operation request: {:?}",
            request_id
        );
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
    let operation_approval = web::block::<_, _, diesel::result::Error>(move || {
        let new_operation_approval = DBNewOperationApproval {
            keyholder_id,
            operation_request_id: operation_approval.operation_request_id,
            signature: operation_approval.signature,
        };

        DBOperationApproval::insert(&conn, new_operation_approval)
    })
    .await?;

    info!(
        "Uploaded signature for operation: {:?} from keyholder: {:?}",
        operation_approval.operation_request_id, keyholder_id
    );

    Ok(operation_approval)
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

async fn find_keyholder_and_validate_signature(
    pool: &web::Data<DbPool>,
    message: &SignableMessage,
    contract: &Contract,
    operation_approval: &NewOperationApproval,
) -> Result<User, APIError> {
    let conn = pool.get()?;
    let contract_id = contract.id.clone();

    let keyholders = web::block::<_, _, APIError>(move || {
        Ok(User::get_all_active(
            &conn,
            contract_id,
            UserKind::Keyholder,
        )?)
    })
    .await?;

    let hashed = message.blake2b_hash()?;
    let filtered_keyholders: Vec<User> = keyholders
        .into_iter()
        .filter(|keyholder| {
            match keyholder.verify_message(&hashed, &operation_approval.signature) {
                Ok(value) => value,
                Err(_) => false,
            }
        })
        .collect();

    if filtered_keyholders.len() == 1 {
        return Ok(filtered_keyholders[0].clone());
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
