use actix_web::{HttpResponse, web};

mod get;
mod post;

pub fn api_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/operations")
            .route(web::get().to(get::get_operations))
            .route(web::post().to(post::post_operations))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
}
