use crate::permissions::verify_site_admin;
use crate::result::ApiResult;
use actix_web::{delete, get, put, web, HttpResponse};
use platz_auth::ApiIdentity;
use platz_db::{Deployment, K8sCluster, K8sClusterFilters, Paginated, UpdateK8sCluster};
use serde_json::json;
use uuid::Uuid;

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Kubernetes Clusters",
    operation_id = "allK8sClusters",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    params(K8sClusterFilters),
    responses(
        (
            status = OK,
            body = inline(Paginated<K8sCluster>),
        ),
    ),
)]
#[get("/k8s-clusters")]
async fn get_all(_identity: ApiIdentity, filters: web::Query<K8sClusterFilters>) -> ApiResult {
    Ok(HttpResponse::Ok().json(K8sCluster::all_filtered(filters.into_inner()).await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Kubernetes Clusters",
    operation_id = "getK8sCluster",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    responses(
        (
            status = OK,
            body = K8sCluster,
        ),
    ),
)]
#[get("/k8s-clusters/{id}")]
async fn get_one(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(K8sCluster::find(id.into_inner()).await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Kubernetes Clusters",
    operation_id = "updateK8sCluster",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    request_body = UpdateK8sCluster,
    responses(
        (
            status = OK,
            body = K8sCluster,
        ),
    ),
)]
#[put("/k8s-clusters/{id}")]
async fn update(
    identity: ApiIdentity,
    id: web::Path<Uuid>,
    data: web::Json<UpdateK8sCluster>,
) -> ApiResult {
    verify_site_admin(&identity).await?;
    let id = id.into_inner();
    let data = data.into_inner();
    Ok(HttpResponse::Ok().json(data.save(id).await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Kubernetes Clusters",
    operation_id = "deleteK8sCluster",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    responses(
        (
            status = NO_CONTENT,
        ),
    ),
)]
#[delete("/k8s-clusters/{id}")]
async fn delete(identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    verify_site_admin(&identity).await?;
    let cluster = K8sCluster::find(id.into_inner()).await?;
    if !Deployment::find_by_cluster_id(cluster.id).await?.is_empty() {
        Ok(HttpResponse::Conflict().json(json!({
            "error": "This cluster has deployments, please delete or move them to another cluster first",
        })))
    } else {
        cluster.delete().await?;
        Ok(HttpResponse::NoContent().finish())
    }
}

#[derive(utoipa::OpenApi)]
#[openapi(
    tags((
        name = "Kubernetes Clusters",
        description = "\
This collection contains Kubernetes clusters detected by Plaz.
        ",
    )),
    paths(get_all, get_one, update, delete),
    components(schemas(
        K8sCluster,
        UpdateK8sCluster,
    )),
)]
pub(super) struct OpenApi;
