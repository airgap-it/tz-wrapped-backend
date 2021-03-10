use actix_web::{web, HttpResponse};

mod delete;
mod get;
mod patch;
mod post;

pub fn api_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/operation-requests")
            .route(web::get().to(get::operation_requests))
            .route(web::post().to(post::operation_request))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
    cfg.service(
        web::resource("/operation-requests/{id}")
            .route(web::get().to(get::operation_request))
            .route(web::patch().to(patch::operation_request))
            .route(web::delete().to(delete::operation_request))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
    cfg.service(
        web::resource("/operation-requests/{id}/signable-message")
            .route(web::get().to(get::signable_message))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
    cfg.service(
        web::resource("/operation-requests/{id}/parameters")
            .route(web::get().to(get::operation_request_parameters))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
}
