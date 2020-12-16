use actix_web::{web, HttpResponse};

mod get;

pub fn api_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/users")
            .route(web::get().to(get::get_users))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
    cfg.service(
        web::resource("/users/{id}")
            .route(web::get().to(get::get_user))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
}
