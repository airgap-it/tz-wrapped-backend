use actix_web::{web, HttpResponse};

mod get;
mod post;

pub fn api_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/nodes")
            .route(web::get().to(get::nodes))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
    cfg.service(
        web::resource("/nodes/selected")
            .route(web::get().to(get::selected_node))
            .route(web::post().to(post::mark_selected))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
}
