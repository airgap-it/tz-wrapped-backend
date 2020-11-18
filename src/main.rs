use actix_web::{web, App, HttpServer, Responder};
use actix_cors::Cors;

mod crypto;
mod tezos;
mod api;

async fn index() -> impl Responder {
    "Hello world!"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        let cors = Cors::default()
        .allow_any_origin()
        .allow_any_method()
        .allow_any_header();
        App::new()
        .wrap(cors)
        .service(
            // prefixes all resources and routes attached to it...
            web::scope("/app")
            .route("/index.html", web::get().to(index)),
        ).service(
            web::scope("/api/v1")
            .configure(api::operation::mint::api_config),
        )
    })
    .bind("0.0.0.0:80")?
    .run()
    .await
}