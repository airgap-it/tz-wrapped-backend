use std::str::FromStr;

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
use num_bigint::BigInt;

use crate::{
    api::models::operation_request::{NewOperationRequest, OperationRequestKind},
    db::models::operation_request::OperationRequest,
    CONFIG,
};
use crate::{
    api::models::{common::SignableMessageInfo, error::APIError},
    db::models::{contract::Contract, user::User},
};

pub fn notify_new_operation_request(
    gatekeeper: &User,
    keyholders: &Vec<User>,
    operation_request: &NewOperationRequest,
    signable_message: &SignableMessageInfo,
    contract: &Contract,
) -> Result<(), APIError> {
    let destinations = keyholders
        .iter()
        .flat_map(|user| user.email.clone())
        .collect();

    let amount = BigInt::from_str(&operation_request.amount)?;
    let human_readable_amount = BigDecimal::new(amount, contract.decimals.into()).to_string();
    let target_address_line: String;
    if let Some(target_address) = operation_request.target_address.as_ref() {
        target_address_line = format!("<b>To:</b> {}", target_address)
    } else {
        target_address_line = "".into();
    }
    send_email(
        destinations,
        format!(
            "New {} {} operation request",
            contract.display_name, operation_request.kind
        ),
        format!(
"\
<html>
<head/>
<body>
<p>
A new {} operation request for {} is waiting for approval.<br>
<br>
<b>Created by:</b> {}<br>
<b>Kind:</b> {}<br>
<b>Amount:</b> {} {}<br>
{}<br>
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
            operation_request.kind,
            contract.display_name,
            gatekeeper.display_name,
            operation_request.kind,
            human_readable_amount.trim_end_matches("0").trim_end_matches("."),
            contract.display_name,
            target_address_line,
            signable_message.tezos_client_command,
            signable_message.message,
            signable_message.blake2b_hash
        ),
    )
}

pub fn notify_min_approvals_received(
    user: &User,
    kind: OperationRequestKind,
    operation_request: &OperationRequest,
    contract: &Contract,
) -> Result<(), APIError> {
    if let Some(to_email) = user.email.as_ref() {
        let human_readable_amount = BigDecimal::new(
            operation_request.amount.as_bigint_and_exponent().0,
            contract.decimals.into(),
        )
        .to_string();
        let target_address_line: String;
        if let Some(target_address) = operation_request.target_address.as_ref() {
            target_address_line = format!("<b>To:</b> {}", target_address)
        } else {
            target_address_line = "".into();
        }

        send_email(
            vec![to_email.clone()],
            format!("{} operation request approved", contract.display_name),
            format!(
                "\
<html>
<head/>
<body>
<p>
The {} operation request for {} has been approved and it is ready to be injected.<br>
<br>
<b>Kind:</b> {}<br>
<b>Amount:</b> {} {}<br>
{}<br>
</p>
</body>
</html>
",
                kind,
                contract.display_name,
                kind,
                human_readable_amount
                    .trim_end_matches("0")
                    .trim_end_matches("."),
                contract.display_name,
                target_address_line
            ),
        )
    } else {
        Ok(())
    }
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
