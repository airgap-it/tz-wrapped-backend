use actix_web::{web, HttpResponse};

mod delete;
mod get;
mod patch;
mod post;

pub fn api_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/auth")
            .route(web::get().to(get::sign_in))
            .route(web::post().to(post::sign_in))
            .route(web::delete().to(delete::sign_out))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
    cfg.service(
        web::resource("/auth/me")
            .route(web::get().to(get::me))
            .route(web::patch().to(patch::me))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
}
