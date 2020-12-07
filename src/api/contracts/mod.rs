use actix_web::{HttpResponse, web};

mod get;

pub fn api_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/contracts")
            .route(web::get().to(get::get_contracts))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
}