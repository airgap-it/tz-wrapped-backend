#![allow(dead_code)]

use actix_cors::Cors;
use actix_session::CookieSession;
use actix_web::{cookie::SameSite, http::Uri, middleware, web, App, HttpServer, Responder};

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
use crypto::generate_random_bytes;
use db::models::contract;
use db::models::node_endpoint;
use db::models::user;
use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;
use diesel_migrations::embed_migrations;
use dotenv::dotenv;
use r2d2::PooledConnection;
use settings::ENV;
use user::SyncUser;

mod api;
mod auth;
mod crypto;
mod db;
mod notifications;
mod settings;
mod tezos;

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;
type Conn = PooledConnection<ConnectionManager<PgConnection>>;

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

async fn health() -> impl Responder {
    "Hello world!"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info,actix_web::middleware::logger=warn");
    env_logger::Builder::from_default_env()
        .target(env_logger::Target::Stdout)
        .init();

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

    let key = generate_random_bytes(32);
    HttpServer::new(move || {
        let secure = CONFIG.env != ENV::Local;
        let same_site = if CONFIG.env == ENV::Production || !secure {
            SameSite::Lax
        } else {
            SameSite::None // this allows to run the frontend on localhost and connect to the DEV instance
        };
        let session = CookieSession::private(&key)
            .secure(secure)
            .domain(CONFIG.server.domain_name.clone())
            .path("/")
            .http_only(true)
            .same_site(same_site);

        let domain_suffix: &str = domain_suffix();
        let allowed_origins: Vec<(&str, &str)> = match CONFIG.env {
            ENV::Development => vec![("http", "localhost"), ("https", domain_suffix)], // this allows to run the frontend on localhost and connect to the DEV instance
            ENV::Testing => vec![],
            ENV::Production => vec![("https", domain_suffix)],
            ENV::Local => vec![("http", domain_suffix), ("https", domain_suffix)],
        };
        let cors = Cors::default()
            .allowed_origin_fn(move |origin, _req_header| {
                let url = origin.to_str().unwrap().parse::<Uri>().unwrap();

                allowed_origins.iter().any(|allowed_origin| {
                    url.scheme_str().unwrap() == allowed_origin.0
                        && url.host().unwrap().ends_with(allowed_origin.1)
                })
            })
            .allow_any_method()
            .allow_any_header()
            .supports_credentials();
        App::new()
            .data(pool.clone())
            .wrap(middleware::Logger::default())
            .wrap(session)
            .wrap(cors)
            .wrap(middleware::Compress::default())
            .route("/", web::get().to(health))
            .service(
                web::scope("/api/v1")
                    .data(CONFIG.server.clone())
                    .data(CONFIG.contracts.clone())
                    .configure(api::contracts::api_config)
                    .configure(api::users::api_config)
                    .configure(api::operation_requests::api_config)
                    .configure(api::operation_approvals::api_config)
                    .configure(api::authentication::api_config)
                    .configure(api::nodes::api_config),
            )
    })
    .bind(&CONFIG.server.address)?
    .run()
    .await
}

fn domain_suffix() -> &'static str {
    let server_domain_name = &CONFIG.server.domain_name;
    let index = server_domain_name.find(".");

    if let Some(index) = index {
        &server_domain_name[index..]
    } else {
        &server_domain_name[..]
    }
}

async fn sync_db(pool: &DbPool) -> Result<(), APIError> {
    log::info!("syncing DB");
    let contracts = &CONFIG.contracts;
    let mut conn = pool.get()?;
    node_endpoint::NodeEndpoint::sync(&conn, &CONFIG.tezos_nodes)?;
    let node_url = node_endpoint::NodeEndpoint::get_selected(&conn)?.url;
    contract::Contract::sync_contracts(pool, contracts, &node_url).await?;
    let stored_contracts =
        web::block::<_, _, APIError>(move || Ok(contract::Contract::get_all(&conn)?)).await?;

    for contract in contracts {
        let gatekeepers = &contract.gatekeepers;
        let stored_contract = stored_contracts.iter().find(|stored_contract| {
            stored_contract.pkh == contract.address
                && stored_contract.multisig_pkh == contract.multisig
                && (stored_contract.token_id as i64) == contract.token_id
        });
        if let Some(stored_contract) = stored_contract {
            conn = pool.get()?;
            let stored_contract_id = stored_contract.id.clone();
            web::block::<_, _, APIError>(move || {
                if let Some(admins) = &CONFIG.server.admins {
                    user::User::sync_users(
                        &conn,
                        stored_contract_id,
                        UserKind::Admin,
                        admins
                            .into_iter()
                            .map(|admin| SyncUser {
                                public_key: admin.public_key.clone(),
                                display_name: admin.name.clone().unwrap_or("".into()),
                                email: admin.email.clone(),
                            })
                            .collect::<Vec<SyncUser>>()
                            .as_ref(),
                    )?;
                }
                user::User::sync_users(
                    &conn,
                    stored_contract_id,
                    UserKind::Gatekeeper,
                    gatekeepers
                        .into_iter()
                        .map(|gatekeeper| SyncUser {
                            public_key: gatekeeper.public_key.clone(),
                            display_name: gatekeeper.name.clone().unwrap_or("".into()),
                            email: gatekeeper.email.clone(),
                        })
                        .collect::<Vec<SyncUser>>()
                        .as_ref(),
                )?;

                Ok(())
            })
            .await?;
        }
    }

    db::sync_keyholders(pool, stored_contracts, &node_url).await?;

    log::info!("syncing DB done");
    Ok(())
}
