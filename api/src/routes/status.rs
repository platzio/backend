use crate::result::ApiResult;
use actix_web::{web, HttpResponse};

async fn get() -> ApiResult {
    Ok(HttpResponse::Ok().json("ok"))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(get));
}
