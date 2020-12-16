use std::convert::TryFrom;

use actix_web::{web, HttpResponse};
use diesel::prelude::*;
use uuid::Uuid;

use crate::{
    api::models::operations::OperationKind,
    crypto,
    db::models::{
        operation_approval::OperationApproval,
        user::{SyncUser, User},
    },
};
use crate::{api::models::users::UserKind, tezos::contract::get_signable_message};
use crate::{
    api::models::{
        approvals::{OperationApprovalResponse, PostOperationApprovalBody},
        error::APIError,
    },
    tezos::contract::multisig::Multisig,
};
use crate::{
    db::models::{
        contract::Contract, operation_approval::NewOperationApproval,
        operation_request::OperationRequest,
    },
    DbPool,
};
use crate::{
    db::schema::{contracts, operation_approvals, operation_requests},
    settings,
};

pub async fn post_approval(
    pool: web::Data<DbPool>,
    tezos_settings: web::Data<settings::Tezos>,
    contract_settings: web::Data<Vec<settings::Contract>>,
    body: web::Json<PostOperationApprovalBody>,
) -> Result<HttpResponse, APIError> {
    let (operation_request, contract) =
        get_operation_request_and_contract(&pool, body.request).await?;

    let mut multisig = Multisig::new(
        contract.multisig_pkh.as_ref(),
        tezos_settings.node_url.as_ref(),
    );

    let message = get_signable_message(
        &contract,
        OperationKind::try_from(operation_request.kind)?,
        operation_request.target_address.as_ref(),
        operation_request.amount,
        operation_request.nonce.into(),
        operation_request.chain_id.as_ref(),
        &multisig,
    )
    .await?;

    let min_approvals = multisig.min_signatures().await?;

    let keyholder = find_and_validate_keyholder(
        &pool,
        message,
        &contract,
        &mut multisig,
        &body,
        contract_settings,
    )
    .await?;

    let inserted_approval = store_approval(&pool, keyholder.id, body.into_inner()).await?;

    let result = OperationApprovalResponse::from(inserted_approval, keyholder)?;

    let request_id = operation_request.id;
    let mut conn = pool.get()?;
    let total_approvals = web::block(move || OperationApproval::count(&conn, &request_id)).await?;

    conn = pool.get()?;
    if total_approvals >= min_approvals {
        web::block(move || OperationRequest::mark_approved(&conn, &request_id)).await?;
    }

    Ok(HttpResponse::Ok().json(result))
}

async fn store_approval(
    pool: &web::Data<DbPool>,
    keyholder_id: Uuid,
    operation_approval: PostOperationApprovalBody,
) -> Result<OperationApproval, APIError> {
    let conn = pool.get()?;
    Ok(web::block::<_, _, diesel::result::Error>(move || {
        let approval = NewOperationApproval {
            approver: keyholder_id,
            request: operation_approval.request,
            kh_signature: operation_approval.kh_signature,
        };

        let inserted_approval: OperationApproval =
            diesel::insert_into(operation_approvals::dsl::operation_approvals)
                .values(approval)
                .get_result(&conn)?;

        Ok(inserted_approval)
    })
    .await?)
}

async fn find_and_validate_keyholder<'a>(
    pool: &web::Data<DbPool>,
    message: String,
    contract: &'a Contract,
    multisig: &'a mut Multisig<'a>,
    operation_approval: &PostOperationApprovalBody,
    contract_settings: web::Data<Vec<settings::Contract>>,
) -> Result<User, APIError> {
    let keyholder_public_keys = multisig.approvers().await?.to_owned();

    let contract_setting = contract_settings
        .iter()
        .find(|contract_setting| {
            contract_setting.address == contract.pkh
                && contract_setting.multisig == contract.multisig_pkh
                && contract_setting.token_id == (contract.token_id as i64)
        })
        .expect("corresponding contract settings must be found");

    let keyholders: Vec<_> = keyholder_public_keys
        .into_iter()
        .enumerate()
        .map(|(position, public_key)| {
            let keyholder_settings = if position < contract_setting.keyholders.len() {
                Some(&contract_setting.keyholders[position])
            } else {
                None
            };

            SyncUser {
                public_key,
                display_name: keyholder_settings
                    .map(|kh| kh.name.clone())
                    .unwrap_or(String::from("Unknown")),
                email: keyholder_settings.map(|kh| kh.email.clone()),
            }
        })
        .collect();

    let conn = pool.get()?;
    let contract_id = contract.id.clone();
    let keyholders = web::block::<_, _, APIError>(move || {
        let _changes =
            User::sync_users(&conn, contract_id, UserKind::Keyholder, keyholders.as_ref())?;

        Ok(User::get_active(&conn, contract_id, UserKind::Keyholder)?)
    })
    .await?;

    let message_bytes = hex::decode(message).map_err(|_error| APIError::InvalidValue {
        description: String::from("expected valid hex value"),
    })?;

    let hashed = crypto::generic_hash(&message_bytes, 32).map_err(|_error| APIError::Internal {
        description: String::from("hash failure"),
    })?;

    let mut result: Result<User, APIError> = Err(APIError::InvalidSignature);
    for kh in keyholders {
        let is_match = kh.verify_message(&hashed, &operation_approval.kh_signature)?;
        if is_match {
            result = Ok(kh);
            break;
        }
    }

    result
}

async fn get_operation_request_and_contract(
    pool: &web::Data<DbPool>,
    operation_request_id: Uuid,
) -> Result<(OperationRequest, Contract), APIError> {
    let conn = pool.get()?;

    let result: (OperationRequest, Contract) = web::block(move || {
        operation_requests::dsl::operation_requests
            .find(operation_request_id)
            .inner_join(contracts::table)
            .first(&conn)
    })
    .await?;

    Ok(result)
}
