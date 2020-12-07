use actix_web::{HttpResponse, web};

mod get;

pub fn api_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/approvals")
            .route(web::get().to(get::get_approvals))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
}
