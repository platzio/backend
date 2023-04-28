use crate::permissions::verify_site_admin;
use crate::result::ApiResult;
use actix_web::{get, put, web, HttpResponse};
use platz_auth::ApiIdentity;
use platz_db::{HelmRegistry, HelmRegistryFilters, UpdateHelmRegistry};
use uuid::Uuid;

#[get("/helm-registries")]
async fn get_all(_identity: ApiIdentity, filters: web::Query<HelmRegistryFilters>) -> ApiResult {
    Ok(HttpResponse::Ok().json(HelmRegistry::all_filtered(filters.into_inner()).await?))
}

#[get("/helm-registries/{id}")]
async fn get_one(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(HelmRegistry::find(id.into_inner()).await?))
}

#[put("/helm-registries/{id}")]
async fn update(
    identity: ApiIdentity,
    id: web::Path<Uuid>,
    data: web::Json<UpdateHelmRegistry>,
) -> ApiResult {
    verify_site_admin(&identity).await?;
    let id = id.into_inner();
    let data = data.into_inner();
    Ok(HttpResponse::Ok().json(data.save(id).await?))
}
