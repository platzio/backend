use crate::permissions::verify_site_admin;
use crate::result::ApiResult;
use actix_web::{delete, get, put, web, HttpResponse};
use platz_auth::ApiIdentity;
use platz_db::{Deployment, K8sCluster, K8sClusterFilters, UpdateK8sCluster};
use serde_json::json;
use uuid::Uuid;

#[get("/k8s-clusters")]
async fn get_all(_identity: ApiIdentity, filters: web::Query<K8sClusterFilters>) -> ApiResult {
    Ok(HttpResponse::Ok().json(K8sCluster::all_filtered(filters.into_inner()).await?))
}

#[get("/k8s-clusters/{id}")]
async fn get_one(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(K8sCluster::find(id.into_inner()).await?))
}

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
