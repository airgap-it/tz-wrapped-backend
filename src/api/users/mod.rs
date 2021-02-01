use actix_web::{web, HttpResponse};

mod get;

pub fn api_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/users")
            .route(web::get().to(get::users))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
    cfg.service(
        web::resource("/users/{id}")
            .route(web::get().to(get::user))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
}
