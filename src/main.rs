#![allow(dead_code)]
use std::convert::TryInto;

use actix_cors::Cors;
use actix_session::CookieSession;
use actix_web::{middleware, web, App, HttpServer, Responder};

#[macro_use]
extern crate diesel;
extern crate dotenv;
#[macro_use]
extern crate diesel_migrations;
// #[macro_use]
extern crate num_derive;
#[macro_use]
extern crate lazy_static;
extern crate env_logger;
extern crate lettre;
extern crate lettre_email;
extern crate native_tls;

use api::models::{error::APIError, user::UserKind};
use db::models::contract;
use db::models::user;
use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;
use diesel_migrations::embed_migrations;
use dotenv::dotenv;
use settings::ENV;
use user::SyncUser;
// use std::env;

mod api;
mod auth;
mod crypto;
mod db;
mod notifications;
mod settings;
mod tezos;

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

embed_migrations!("./migrations");

lazy_static! {
    static ref CONFIG: settings::Settings =
        settings::Settings::new().expect("config can be loaded");
}

fn database_url() -> String {
    dotenv().ok();
    let user = &CONFIG.database.user;
    let password = &CONFIG.database.password;
    let host = &CONFIG.database.host;
    let name = &CONFIG.database.name;

    format!("postgres://{}:{}@{}:5432/{}", user, password, host, name)
}

async fn index() -> impl Responder {
    "Hello world!"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info,actix_server=info");
    env_logger::init();

    let database_url = database_url();
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");

    let _result = embedded_migrations::run_with_output(
        &pool
            .get()
            .expect("Failed to get a connection from the pool"),
        &mut std::io::stdout(),
    );

    sync_db(&pool)
        .await
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))?;

    HttpServer::new(move || {
        let session = CookieSession::signed(&[0; 32])
            .secure(CONFIG.env != ENV::Local)
            .domain(CONFIG.server.domain_name.clone())
            .path("/")
            .http_only(true)
            .expires_in(86400);
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .supports_credentials();
        App::new()
            .data(pool.clone())
            .wrap(middleware::Logger::default())
            .wrap(session)
            .wrap(cors)
            .wrap(middleware::Compress::default())
            .route("/", web::get().to(index))
            .service(
                web::scope("/api/v1")
                    .data(CONFIG.tezos.clone())
                    .configure(api::operation_requests::api_config)
                    .configure(api::users::api_config)
                    .configure(api::contracts::api_config)
                    .data(CONFIG.contracts.clone())
                    .configure(api::operation_approvals::api_config)
                    .data(CONFIG.server.clone())
                    .configure(api::authentication::api_config),
            )
    })
    .bind(&CONFIG.server.address)?
    .run()
    .await
}

async fn sync_db(pool: &DbPool) -> Result<(), APIError> {
    println!("syncing DB");
    let contracts = &CONFIG.contracts;
    let mut conn = pool.get()?;
    contract::Contract::sync_contracts(pool, contracts, &CONFIG.tezos.node_url).await?;
    let stored_contracts =
        web::block::<_, _, APIError>(move || Ok(contract::Contract::get_all(&conn)?)).await?;

    for contract in contracts {
        let gatekeepers = &contract.gatekeepers;
        let stored_contract = stored_contracts.iter().find(|stored_contract| {
            stored_contract.pkh == contract.address
                && (stored_contract.token_id as i64) == contract.token_id
        });

        if let Some(stored_contract) = stored_contract {
            conn = pool.get()?;
            let stored_contract_id = stored_contract.id.clone();
            web::block::<_, _, APIError>(move || {
                let _changes = user::User::sync_users(
                    &conn,
                    stored_contract_id,
                    UserKind::Gatekeeper,
                    gatekeepers
                        .into_iter()
                        .map(|gatekeeper| SyncUser {
                            public_key: gatekeeper.public_key.clone(),
                            display_name: gatekeeper.name.clone(),
                            email: Some(gatekeeper.email.clone()),
                        })
                        .collect::<Vec<SyncUser>>()
                        .as_ref(),
                )?;

                Ok(())
            })
            .await?;
        }
    }

    for contract in stored_contracts {
        let mut multisig = tezos::contract::multisig::get_multisig(
            contract.multisig_pkh.as_ref(),
            contract.kind.try_into()?,
            CONFIG.tezos.node_url.as_ref(),
        );

        let contract_settings = contracts
            .iter()
            .find(|contract_settings| {
                contract_settings.address == contract.pkh
                    && contract_settings.multisig == contract.multisig_pkh
                    && contract_settings.token_id == (contract.token_id as i64)
            })
            .expect("corresponding contract settings must be found");

        let keyholders: Vec<_> = multisig
            .approvers()
            .await?
            .into_iter()
            .enumerate()
            .map(|(position, public_key)| {
                let keyholder_settings = if position < contract_settings.keyholders.len() {
                    Some(&contract_settings.keyholders[position])
                } else {
                    None
                };

                SyncUser {
                    public_key: public_key.clone(),
                    display_name: keyholder_settings
                        .map(|kh| kh.name.clone())
                        .unwrap_or("Unknown".into()),
                    email: keyholder_settings.map(|kh| kh.email.clone()),
                }
            })
            .collect();
        conn = pool.get()?;
        web::block::<_, _, APIError>(move || {
            let _changes = user::User::sync_users(
                &conn,
                contract.id,
                UserKind::Keyholder,
                keyholders.as_ref(),
            )?;

            Ok(())
        })
        .await?;
    }

    println!("syncing DB done");
    Ok(())
}
