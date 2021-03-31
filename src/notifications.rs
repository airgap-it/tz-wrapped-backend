use std::convert::TryInto;

use bigdecimal::BigDecimal;
use lettre::smtp::ConnectionReuseParameters;
use lettre::ClientSecurity;
use lettre::{
    smtp::authentication::{Credentials, Mechanism},
    ClientTlsParameters,
};
use lettre::{SmtpClient, Transport};
use lettre_email::Email;
use native_tls::{Protocol, TlsConnector};

use crate::{
    api::models::operation_request::OperationRequestKind,
    db::models::operation_request::OperationRequest, CONFIG,
};
use crate::{
    api::models::{common::SignableMessageInfo, error::APIError},
    db::models::{contract::Contract, user::User},
};

pub fn notify_new_operation_request(
    user: &User,
    keyholders: &Vec<User>,
    operation_request: &OperationRequest,
    signable_message: &SignableMessageInfo,
    contract: &Contract,
) -> Result<(), APIError> {
    let destinations = keyholders
        .iter()
        .filter(|keyholder| keyholder.id != user.id)
        .flat_map(|user| user.email.clone())
        .collect::<Vec<_>>();

    if destinations.is_empty() {
        return Ok(());
    }

    let amount_line = amount_line(operation_request, contract);
    let target_address_line = target_address_line(operation_request);
    let operation_request_kind: OperationRequestKind = operation_request.kind.try_into()?;
    send_email(
        destinations,
        format!(
            "{}: New {} operation request #{}",
            contract.display_name, operation_request_kind, operation_request.nonce
        ),
        format!(
"\
<html>
<head/>
<body>
<p>
A new {} operation request #{} for {} is waiting for approval.<br>
<br>
<b>Created by:</b> {}<br>
<b>Kind:</b> {}<br>
{}
{}
<br>
To reproduce the hash shown by the ledger when approving this operation, use the following tezos-client command.<br>
<pre>{}</pre>
<br>
The output of the above command should show the following data.<br>
<br>
<b>Raw packed data:</b><br>
<pre>0x{}</pre><br>
<b>Ledger Blake2b hash:</b><br>
<pre>{}</pre><br>
</p>
</body>
</html>
",
            operation_request_kind,
            operation_request.nonce,
            contract.display_name,
            if !user.display_name.is_empty() { &user.display_name } else { &user.address },
            operation_request_kind,
            amount_line,
            target_address_line,
            signable_message.tezos_client_command,
            signable_message.message,
            signable_message.blake2b_hash
        ),
    )
}

pub fn notify_approval_received(
    user: &User,
    approver: &User,
    keyholders: &Vec<User>,
    operation_request: &OperationRequest,
    contract: &Contract,
) -> Result<(), APIError> {
    let mut destinations = keyholders
        .iter()
        .flat_map(|keyholder| {
            if keyholder.id == approver.id || keyholder.id == user.id {
                return None;
            }
            keyholder.email.clone()
        })
        .collect::<Vec<_>>();
    if let Some(user_email) = user.email.as_ref() {
        destinations.push(user_email.clone())
    }
    if destinations.is_empty() {
        return Ok(());
    }

    let amount_line = amount_line(operation_request, contract);
    let target_address_line = target_address_line(operation_request);
    let operation_request_kind: OperationRequestKind = operation_request.kind.try_into()?;
    send_email(
        destinations,
        format!(
            "{}: {} operation request #{} recieved an approval",
            contract.display_name, operation_request_kind, operation_request.nonce
        ),
        format!(
            "\
<html>
<head/>
<body>
<p>
The {} operation request #{} for {} has received an approval from {}.<br>
<br>
<b>Created by:</b> {}<br>
<b>Kind:</b> {}<br>
{}
{}
</p>
</body>
</html>
",
            operation_request_kind,
            operation_request.nonce,
            contract.display_name,
            if !approver.display_name.is_empty() {
                &approver.display_name
            } else {
                &approver.address
            },
            if !user.display_name.is_empty() {
                &user.display_name
            } else {
                &user.address
            },
            operation_request_kind,
            amount_line,
            target_address_line
        ),
    )
}

pub fn notify_min_approvals_received(
    user: &User,
    keyholders: &Vec<User>,
    operation_request: &OperationRequest,
    contract: &Contract,
) -> Result<(), APIError> {
    let mut destinations = keyholders
        .iter()
        .filter(|keyholder| keyholder.id != user.id)
        .flat_map(|keyholder| keyholder.email.clone())
        .collect::<Vec<_>>();
    if let Some(user_email) = user.email.as_ref() {
        destinations.push(user_email.clone())
    }
    if destinations.is_empty() {
        return Ok(());
    }
    let amount_line = amount_line(operation_request, contract);
    let target_address_line = target_address_line(operation_request);
    let operation_request_kind: OperationRequestKind = operation_request.kind.try_into()?;
    send_email(
        destinations,
        format!(
            "{}: {} operation request #{} fully approved",
            contract.display_name, operation_request_kind, operation_request.nonce
        ),
        format!(
            "\
<html>
<head/>
<body>
<p>
The {} operation request #{} for {} has been approved and it is ready to be injected.<br>
<br>
<b>Created by:</b> {}<br>
<b>Kind:</b> {}<br>
{}
{}
</p>
</body>
</html>
",
            operation_request_kind,
            operation_request.nonce,
            contract.display_name,
            if !user.display_name.is_empty() {
                &user.display_name
            } else {
                &user.address
            },
            operation_request_kind,
            amount_line,
            target_address_line
        ),
    )
}

pub fn notify_injection(
    user: &User,
    keyholders: &Vec<User>,
    operation_request: &OperationRequest,
    contract: &Contract,
) -> Result<(), APIError> {
    let mut destinations = keyholders
        .iter()
        .filter(|keyholder| keyholder.id != user.id)
        .flat_map(|keyholder| keyholder.email.clone())
        .collect::<Vec<_>>();
    if let Some(user_email) = user.email.as_ref() {
        destinations.push(user_email.clone())
    }
    if destinations.is_empty() {
        return Ok(());
    }
    let amount_line = amount_line(operation_request, contract);
    let target_address_line = target_address_line(operation_request);
    let operation_hash_line = operation_hash_line(operation_request);
    let operation_request_kind: OperationRequestKind = operation_request.kind.try_into()?;
    send_email(
        destinations,
        format!(
            "{}: {} operation request #{} injected",
            contract.display_name, operation_request_kind, operation_request.nonce
        ),
        format!(
            "\
<html>
<head/>
<body>
<p>
The {} operation request #{} for {} has been injected.<br>
<br>
<b>Created by:</b> {}<br>
<b>Kind:</b> {}<br>
{}
{}
{}
</p>
</body>
</html>
",
            operation_request_kind,
            operation_request.nonce,
            contract.display_name,
            if !user.display_name.is_empty() {
                &user.display_name
            } else {
                &user.address
            },
            operation_request_kind,
            amount_line,
            target_address_line,
            operation_hash_line
        ),
    )
}

pub fn send_email(
    destinations: Vec<String>,
    subject: String,
    message: String,
) -> Result<(), APIError> {
    let mut email_builder = Email::builder();
    for destination in destinations {
        email_builder = email_builder.to(destination);
    }
    let email = email_builder
        .from(CONFIG.smtp.user.as_ref())
        .subject(subject)
        .html(message)
        .build()?;

    let mut tls_builder = TlsConnector::builder();
    tls_builder.min_protocol_version(Some(Protocol::Tlsv10));
    let tls_parameters = ClientTlsParameters::new(CONFIG.smtp.host.clone(), tls_builder.build()?);

    let mut mailer = SmtpClient::new(
        (
            &CONFIG.smtp.host[..],
            u16::from_str_radix(&CONFIG.smtp.port, 10)?,
        ),
        ClientSecurity::Required(tls_parameters),
    )?
    .authentication_mechanism(Mechanism::Login)
    .credentials(Credentials::new(
        CONFIG.smtp.user.clone(),
        CONFIG.smtp.password.clone(),
    ))
    .connection_reuse(ConnectionReuseParameters::ReuseUnlimited)
    .transport();

    let _result = mailer.send(email.into());

    Ok(())
}

fn amount_line(operation_request: &OperationRequest, contract: &Contract) -> String {
    let amount = operation_request
        .amount
        .as_ref()
        .map(|amount| amount.as_bigint_and_exponent().0);

    let human_readable_amount =
        amount.map(|amount| BigDecimal::new(amount, contract.decimals.into()).to_string());
    match human_readable_amount {
        Some(amount) => format!(
            "<b>Amount:</b> {} {}<br>",
            amount.trim_end_matches("0").trim_end_matches("."),
            contract.symbol
        ),
        None => "".into(),
    }
}

fn target_address_line(operation_request: &OperationRequest) -> String {
    match operation_request.target_address.as_ref() {
        Some(target_address) => format!("<b>To:</b> {}<br>", target_address),
        None => "".into(),
    }
}

fn operation_hash_line(operation_request: &OperationRequest) -> String {
    match operation_request.operation_hash.as_ref() {
        Some(operation_hash) => format!("<b>Operation Group Hash:</b> {}<br>", operation_hash),
        None => "".into(),
    }
}
