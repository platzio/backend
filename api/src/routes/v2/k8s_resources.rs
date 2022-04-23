use crate::auth::ApiIdentity;
use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use platz_db::{K8sResource, K8sResourceFilters};
use uuid::Uuid;

async fn get_all(_identity: ApiIdentity, filters: web::Query<K8sResourceFilters>) -> ApiResult {
    Ok(HttpResponse::Ok().json(K8sResource::all_filtered(filters.into_inner()).await?))
}

async fn get(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(K8sResource::find(id.into_inner()).await?))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(get_all));
    cfg.route("/{id}", web::get().to(get));
}
