use actix_web::{web, HttpResponse};

mod get;
mod post;

pub fn api_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/operation-approvals")
            .route(web::get().to(get::get_approvals))
            .route(web::post().to(post::post_approval))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
    cfg.service(
        web::resource("/operation-approvals/{id}")
            .route(web::get().to(get::get_approval))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
}
