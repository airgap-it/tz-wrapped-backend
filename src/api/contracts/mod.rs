use actix_web::{web, HttpResponse};

mod get;

pub fn api_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/contracts")
            .route(web::get().to(get::get_contracts))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
    cfg.service(
        web::resource("/contracts/{id}")
            .route(web::get().to(get::get_contract))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
    cfg.service(
        web::resource("/contracts/{id}/nonce")
            .route(web::get().to(get::get_contract_nonce))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
    cfg.service(
        web::resource("/contracts/{id}/signable-message")
            .route(web::get().to(get::get_signable_message))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
}
