use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use platz_db::K8sResource;
use uuid::Uuid;

async fn get_all() -> ApiResult {
    Ok(HttpResponse::Ok().json(K8sResource::all().await?))
}

async fn get(id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(K8sResource::find(id.into_inner()).await?))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(get_all));
    cfg.route("/{id}", web::get().to(get));
}
