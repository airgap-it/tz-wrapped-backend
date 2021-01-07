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
    api::models::error::APIError,
    db::models::{contract::Contract, user::User},
};
use crate::{api::models::operations::OperationKind, CONFIG};

pub fn notify_new_operation_request(
    users: Vec<User>,
    kind: OperationKind,
    contract: Contract,
) -> Result<(), APIError> {
    let destinations = users.iter().flat_map(|user| user.email.clone()).collect();
    send_email(
        destinations,
        format!("New {} {} operation request", contract.display_name, kind),
        format!(
            "A new {} operation request for {} is waiting for approval.",
            kind, contract.display_name
        ),
    )
}

pub fn notify_min_approvals_received(
    user: User,
    kind: OperationKind,
    contract: Contract,
) -> Result<(), APIError> {
    if let Some(to_email) = user.email {
        send_email(
            vec![to_email],
            format!("{} operation request approved", contract.display_name),
            format!(
                "The {} operation request for {} has been approved.",
                contract.display_name, kind
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
        .text(message)
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
