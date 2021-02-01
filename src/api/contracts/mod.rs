use actix_web::{web, HttpResponse};

mod get;

pub fn api_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/contracts")
            .route(web::get().to(get::contracts))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
    cfg.service(
        web::resource("/contracts/{id}")
            .route(web::get().to(get::contract))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
    cfg.service(
        web::resource("/contracts/{id}/nonce")
            .route(web::get().to(get::contract_nonce))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
    cfg.service(
        web::resource("/contracts/{id}/next-usable-nonce")
            .route(web::get().to(get::next_usable_nonce))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
}
