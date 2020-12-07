use actix_web::{HttpResponse, web};

mod get;

pub fn api_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/gatekeepers")
            .route(web::get().to(get::get_gatekeepers))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
}
