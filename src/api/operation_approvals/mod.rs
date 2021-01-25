use actix_web::{web, HttpResponse};

mod get;
mod post;

pub fn api_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/operation-approvals")
            .route(web::get().to(get::operation_approvals))
            .route(web::post().to(post::operation_approval))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
    cfg.service(
        web::resource("/operation-approvals/{id}")
            .route(web::get().to(get::operation_approval))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
}
