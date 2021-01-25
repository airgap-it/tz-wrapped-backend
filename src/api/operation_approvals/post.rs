use std::convert::TryInto;

use actix_session::Session;
use actix_web::{web, HttpResponse};
use multisig::SignableMessage;
use uuid::Uuid;

use crate::api::models::{
    error::APIError,
    operation_approval::{NewOperationApproval, OperationApproval},
};
use crate::db::models::{
    contract::Contract,
    operation_approval::NewOperationApproval as DBNewOperationApproval,
    operation_approval::OperationApproval as DBOperationApproval,
    operation_request::OperationRequest,
    user::{SyncUser, User},
};
use crate::notifications::notify_min_approvals_received;
use crate::settings;
use crate::tezos::contract::{get_signable_message, multisig, multisig::Multisig};
use crate::DbPool;
use crate::{api::models::user::UserKind, auth::get_current_user};

pub async fn operation_approval(
    pool: web::Data<DbPool>,
    tezos_settings: web::Data<settings::Tezos>,
    contract_settings: web::Data<Vec<settings::Contract>>,
    body: web::Json<NewOperationApproval>,
    session: Session,
) -> Result<HttpResponse, APIError> {
    let current_user = get_current_user(&session)?;

    let (operation_request, contract) =
        get_operation_request_and_contract(&pool, body.operation_request_id).await?;

    current_user.require_roles(vec![UserKind::Keyholder], contract.id)?;

    let mut multisig = multisig::get_multisig(
        contract.multisig_pkh.as_ref(),
        contract.kind.try_into()?,
        tezos_settings.node_url.as_ref(),
    );

    let signable_message = get_signable_message(
        &contract,
        operation_request.kind.try_into()?,
        operation_request.target_address.as_ref(),
        operation_request.amount.as_bigint_and_exponent().0,
        operation_request.nonce.into(),
        operation_request.chain_id.as_ref(),
        &multisig,
    )
    .await?;

    let min_approvals = multisig.min_signatures().await?;

    let keyholder = find_and_validate_keyholder(
        &pool,
        &signable_message,
        &contract,
        multisig,
        &body,
        contract_settings,
    )
    .await?;

    let inserted_approval = store_approval(&pool, keyholder.id, body.into_inner()).await?;

    let result = OperationApproval::from(inserted_approval, keyholder)?;

    let request_id = operation_request.id;
    let conn = pool.get()?;
    let total_approvals =
        web::block(move || DBOperationApproval::count(&conn, &request_id)).await?;

    let conn = pool.get()?;
    if total_approvals >= min_approvals {
        web::block(move || {
            let result = OperationRequest::mark_approved(&conn, &request_id);

            if result.is_ok() {
                let gatekeeper = User::get(&conn, operation_request.gatekeeper_id);
                if let Ok(gatekeeper) = gatekeeper {
                    let _notification_result = notify_min_approvals_received(
                        &gatekeeper,
                        operation_request.kind.try_into().unwrap(),
                        &operation_request,
                        &contract,
                    );
                }
            }

            result
        })
        .await?;
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
    message: &SignableMessage,
    contract: &Contract,
    mut multisig: Box<dyn Multisig + '_>,
    operation_approval: &NewOperationApproval,
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
                    .unwrap_or("Unknown".into()),
                email: keyholder_settings.map(|kh| kh.email.clone()),
            }
        })
        .collect();

    let conn = pool.get()?;
    let contract_id = contract.id.clone();
    let keyholders = web::block::<_, _, APIError>(move || {
        let _changes =
            User::sync_users(&conn, contract_id, UserKind::Keyholder, keyholders.as_ref())?;

        Ok(User::get_all_active(
            &conn,
            contract_id,
            UserKind::Keyholder,
        )?)
    })
    .await?;

    let hashed = message.blake2b_hash()?;

    let mut result: Result<User, APIError> = Err(APIError::InvalidSignature);
    for kh in keyholders {
        let is_match = kh.verify_message(&hashed, &operation_approval.signature)?;
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

    let result: (OperationRequest, Contract) =
        web::block(move || OperationRequest::get_with_contract(&conn, &operation_request_id))
            .await?;

    Ok(result)
}
