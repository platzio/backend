use crate::auth::CurIdentity;
use crate::permissions::verify_site_admin;
use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use platz_db::{Deployment, K8sCluster, UpdateK8sCluster};
use serde_json::json;
use uuid::Uuid;

async fn get_all() -> ApiResult {
    Ok(HttpResponse::Ok().json(K8sCluster::all().await?))
}

async fn get(id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(K8sCluster::find(id.into_inner()).await?))
}

async fn update(
    cur_identity: CurIdentity,
    id: web::Path<Uuid>,
    data: web::Json<UpdateK8sCluster>,
) -> ApiResult {
    verify_site_admin(cur_identity.user().id).await?;
    let id = id.into_inner();
    let data = data.into_inner();
    Ok(HttpResponse::Ok().json(data.save(id).await?))
}

async fn delete(cur_identity: CurIdentity, id: web::Path<Uuid>) -> ApiResult {
    verify_site_admin(cur_identity.user().id).await?;
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

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(get_all));
    cfg.route("/{id}", web::get().to(get));
    cfg.route("/{id}", web::put().to(update));
    cfg.route("/{id}", web::delete().to(delete));
}
