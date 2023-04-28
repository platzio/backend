use crate::result::ApiResult;
use actix_web::{get, web, HttpResponse};
use platz_auth::ApiIdentity;
use platz_db::{DeploymentResourceType, DeploymentResourceTypeFilters};
use uuid::Uuid;

#[get("/deployment-resource-types")]
async fn get_all(
    _identity: ApiIdentity,
    filters: web::Query<DeploymentResourceTypeFilters>,
) -> ApiResult {
    Ok(HttpResponse::Ok().json(DeploymentResourceType::all_filtered(filters.into_inner()).await?))
}

#[get("/deployment-resource-types/{id}")]
async fn get_one(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(DeploymentResourceType::find(id.into_inner()).await?))
}
