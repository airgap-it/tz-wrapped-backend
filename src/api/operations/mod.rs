use actix_web::{web, HttpResponse};

mod get;
mod patch;
mod post;

pub fn api_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/operations")
            .route(web::get().to(get::get_operations))
            .route(web::post().to(post::post_operation))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
    cfg.service(
        web::resource("/operations/{id}")
            .route(web::get().to(get::get_operation))
            .route(web::patch().to(patch::patch_operation))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
    cfg.service(
        web::resource("/operations/{id}/signable-message")
            .route(web::get().to(get::get_signable_message))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
    cfg.service(
        web::resource("/operations/{id}/parameters")
            .route(web::get().to(get::get_operation_parameters))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
}
