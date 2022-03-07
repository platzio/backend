use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use platz_db::HelmChart;
use uuid::Uuid;

#[actix_web::get("")]
async fn get_all() -> ApiResult {
    Ok(HttpResponse::Ok().json(HelmChart::all().await?))
}

#[actix_web::get("/{id}")]
async fn get(id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(HelmChart::find(id.into_inner()).await?))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/helm-charts")
            .service(get_all)
            .service(get),
    );
}
