use actix_web::{HttpResponse, Responder, web};

pub fn api_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/operations/mint")
            .route(web::get().to(get_mint_operations))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
}

async fn get_mint_operations() -> impl Responder {
    HttpResponse::Ok().body("Hey there!")
}
