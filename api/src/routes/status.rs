use crate::result::ApiResult;
use actix_web::{web, HttpResponse};

#[actix_web::get("")]
async fn get() -> ApiResult {
    Ok(HttpResponse::Ok().json("ok"))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/api/v1/status").service(get));
}
