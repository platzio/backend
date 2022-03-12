use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use platz_db::DeploymentResourceType;
use uuid::Uuid;

#[actix_web::get("")]
async fn get_all() -> ApiResult {
    Ok(HttpResponse::Ok().json(DeploymentResourceType::all().await?))
}

#[actix_web::get("/{id}")]
async fn get(id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(DeploymentResourceType::find(id.into_inner()).await?))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/deployment-resource-types")
            .service(get_all)
            .service(get),
    );
}
