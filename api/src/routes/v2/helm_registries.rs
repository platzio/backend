use crate::{permissions::verify_site_admin, result::ApiResult};
use actix_web::{get, put, web, HttpResponse};
use platz_auth::ApiIdentity;
use platz_db::{
    diesel_pagination::{Paginated, PaginationParams},
    schema::helm_registry::{HelmRegistry, HelmRegistryFilters, UpdateHelmRegistry},
};
use uuid::Uuid;

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Helm Registries",
    operation_id = "allHelmRegistries",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    params(HelmRegistryFilters),
    responses(
        (
            status = OK,
            body = Paginated<HelmRegistry>,
        ),
    ),
)]
#[get("/helm-registries")]
async fn get_all(
    _identity: ApiIdentity,
    filters: web::Query<HelmRegistryFilters>,
    pagination: web::Query<PaginationParams>,
) -> ApiResult {
    Ok(HttpResponse::Ok()
        .json(HelmRegistry::all_filtered(filters.into_inner(), pagination.into_inner()).await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Helm Registries",
    operation_id = "getHelmRegistry",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    responses(
        (
            status = OK,
            body = HelmRegistry,
        ),
    ),
)]
#[get("/helm-registries/{id}")]
async fn get_one(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(HelmRegistry::find(id.into_inner()).await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Helm Registries",
    operation_id = "updateHelmRegistry",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    request_body = UpdateHelmRegistry,
    responses(
        (
            status = OK,
            body = HelmRegistry,
        ),
    ),
)]
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

#[derive(utoipa::OpenApi)]
#[openapi(
    tags((
        name = "Helm Registries",
        description = "\
This collection contains Helm registries detected by the chart-discovery
service.

New registries are created automatically for Helm chart whenever new charts
are created in those registries.
        ",
    )),
    paths(get_all, get_one, update),
)]
pub(super) struct OpenApi;
