use actix_web::{App, HttpServer, Responder, middleware, web};
use actix_cors::Cors;

#[macro_use]
extern crate diesel;
extern crate dotenv;
#[macro_use]
extern crate diesel_migrations;

use diesel::{r2d2::ConnectionManager};
use diesel::pg::PgConnection;
use diesel_migrations::embed_migrations;
use dotenv::dotenv;
use std::env;

mod crypto;
mod tezos;
mod api;
mod db;

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

embed_migrations!();

fn database_url() -> String {
    dotenv().ok();
    // postgres://DB_USER:DB_PASSWORD@DB_HOST:5432/DB_NAME
    let user = env::var("DB_USER").expect("DB_USER must be set");
    let password = env::var("DB_PASSWORD").expect("DB_PASSWORD must be set");
    let host = env::var("DB_HOST").expect("DB_HOST must be set");
    let name = env::var("DB_NAME").expect("DB_NAME must be set");

    format!("postgres://{}:{}@{}:5432/{}", user, password, host, name)
}

async fn index() -> impl Responder {
    "Hello world!"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let database_url = database_url();
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");

    let _result = embedded_migrations::run(&pool.get().expect("Failed to get a connection from the pool"));
    
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();
        App::new()
            .data(pool.clone())
            .wrap(cors)
            .wrap(middleware::Compress::default())
            .service(
                web::scope("/app")
                            .route("/index.html", web::get().to(index))
            ).service(
                web::scope("/api/v1")
                            .configure(api::operations::api_config)
                            .configure(api::gatekeepers::api_config)
                            .configure(api::contracts::api_config)
                            .configure(api::approvals::api_config)
        )
    })
    .bind("0.0.0.0:80")?
    .run()
    .await
}
