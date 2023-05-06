use crate::result::ApiResult;
use actix_web::{get, web, HttpResponse};
use platz_auth::ApiIdentity;
use platz_db::{K8sResource, K8sResourceFilters, Paginated};
use uuid::Uuid;

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Kubernetes Resources",
    operation_id = "allK8sResources",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    params(K8sResourceFilters),
    responses(
        (
            status = OK,
            body = inline(Paginated<K8sResource>),
        ),
    ),
)]
#[get("/k8s-resources")]
async fn get_all(_identity: ApiIdentity, filters: web::Query<K8sResourceFilters>) -> ApiResult {
    Ok(HttpResponse::Ok().json(K8sResource::all_filtered(filters.into_inner()).await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Kubernetes Resources",
    operation_id = "getK8sResource",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    responses(
        (
            status = OK,
            body = K8sResource,
        ),
    ),
)]
#[get("/k8s-resources/{id}")]
async fn get_one(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(K8sResource::find(id.into_inner()).await?))
}

#[derive(utoipa::OpenApi)]
#[openapi(
    tags((
        name = "Kubernetes Resources",
        description = "\
This collection contains Kubernetes resources of Platz deployments.

Kubernetes resources are automatically tracked in every namespace created by
Platz.
        ",
    )),
    paths(get_all, get_one),
    components(schemas(
        K8sResource,
    )),
)]
pub(super) struct OpenApi;
