use crate::result::ApiResult;
use actix_web::{get, web, HttpResponse};
use platz_auth::ApiIdentity;
use platz_db::{K8sResource, K8sResourceFilters};
use uuid::Uuid;

#[get("/k8s-resources")]
async fn get_all(_identity: ApiIdentity, filters: web::Query<K8sResourceFilters>) -> ApiResult {
    Ok(HttpResponse::Ok().json(K8sResource::all_filtered(filters.into_inner()).await?))
}

#[get("/k8s-resources/{id}")]
async fn get_one(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(K8sResource::find(id.into_inner()).await?))
}
