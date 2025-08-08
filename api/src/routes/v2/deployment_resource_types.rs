use crate::result::ApiResult;
use actix_web::{get, web, HttpResponse};
use platz_auth::ApiIdentity;
use platz_db::{
    diesel_pagination::{Paginated, PaginationParams},
    schema::deployment_resource_type::{DeploymentResourceType, DeploymentResourceTypeFilters},
};
use uuid::Uuid;

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Deployment Resource Types",
    operation_id = "allDeploymentResourceTypes",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    params(DeploymentResourceTypeFilters),
    responses(
        (
            status = OK,
            body = Paginated<DeploymentResourceType>,
        ),
    ),
)]
#[get("/deployment-resource-types")]
async fn get_all(
    _identity: ApiIdentity,
    filters: web::Query<DeploymentResourceTypeFilters>,
    pagination: web::Query<PaginationParams>,
) -> ApiResult {
    Ok(HttpResponse::Ok().json(
        DeploymentResourceType::all_filtered(filters.into_inner(), pagination.into_inner()).await?,
    ))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Deployment Resource Types",
    operation_id = "getDeploymentResourceType",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    responses(
        (
            status = OK,
            body = DeploymentResourceType,
        ),
    ),
)]
#[get("/deployment-resource-types/{id}")]
async fn get_one(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(DeploymentResourceType::find(id.into_inner()).await?))
}

#[derive(utoipa::OpenApi)]
#[openapi(
    tags((
        name = "Deployment Resource Types",
        description = "\
Deployment resource types are custom types defined in a chart's extensions
contained in the `platz` directory in the deployment Helm chart.
        ",
    )),
    paths(get_all, get_one),
)]
pub(super) struct OpenApi;
