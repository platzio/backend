use crate::auth::CurUser;
use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use platz_db::DeploymentResourceType;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct GetAllQuery {
    env_id: Option<Uuid>,
    deployment_kind: Option<String>,
    key: Option<String>,
}

async fn get_all(_cur_user: CurUser, query: web::Query<GetAllQuery>) -> ApiResult {
    Ok(match query.into_inner() {
        GetAllQuery {
            env_id: Some(env_id),
            deployment_kind: None,
            key: None,
        } => HttpResponse::Ok().json(DeploymentResourceType::find_by_env(env_id).await?),
        GetAllQuery {
            env_id: Some(env_id),
            deployment_kind: Some(deployment_kind),
            key: Some(key),
        } => HttpResponse::Ok().json(
            DeploymentResourceType::find_by_env_kind_and_key(env_id, deployment_kind, key).await?,
        ),
        GetAllQuery {
            env_id: None,
            deployment_kind: Some(deployment_kind),
            key: Some(key),
        } => HttpResponse::Ok()
            .json(DeploymentResourceType::find_by_kind_and_key(deployment_kind, key).await?),
        GetAllQuery {
            env_id: None,
            deployment_kind: None,
            key: None,
        } => HttpResponse::Ok().json(DeploymentResourceType::all().await?),
        _ => HttpResponse::BadRequest().json(json!({
            "message": "Invalid query parameter combination"
        })),
    })
}

async fn get(_cur_user: CurUser, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(DeploymentResourceType::find(id.into_inner()).await?))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(get_all));
    cfg.route("/{id}", web::get().to(get));
}
