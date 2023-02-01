pub mod auth;
mod deployment_permissions;
mod deployment_resource_types;
mod deployment_resources;
mod deployment_tasks;
mod deployments;
mod env_user_permissions;
mod envs;
mod helm_charts;
mod helm_registries;
mod helm_tag_formats;
mod k8s_clusters;
mod k8s_resources;
mod secrets;
mod user_tokens;
mod users;
mod ws;

use actix_web::{web, HttpResponse};
use serde::Serialize;

use crate::result::ApiResult;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("/self", web::get().to(self_route));
    cfg.service(web::scope("/auth").configure(auth::config));
    cfg.service(web::scope("/deployment-permissions").configure(deployment_permissions::config));
    cfg.service(
        web::scope("/deployment-resource-types").configure(deployment_resource_types::config),
    );
    cfg.service(web::scope("/deployment-resources").configure(deployment_resources::config));
    cfg.service(web::scope("/deployment-tasks").configure(deployment_tasks::config));
    cfg.service(web::scope("/deployments").configure(deployments::config));
    cfg.service(web::scope("/env-user-permissions").configure(env_user_permissions::config));
    cfg.service(web::scope("/envs").configure(envs::config));
    cfg.service(web::scope("/helm-charts").configure(helm_charts::config));
    cfg.service(web::scope("/helm-registries").configure(helm_registries::config));
    cfg.service(web::scope("/helm-tag-formats").configure(helm_tag_formats::config));
    cfg.service(web::scope("/k8s-clusters").configure(k8s_clusters::config));
    cfg.service(web::scope("/k8s-resources").configure(k8s_resources::config));
    cfg.service(web::scope("/secrets").configure(secrets::config));
    cfg.service(web::scope("/user-tokens").configure(user_tokens::config));
    cfg.service(web::scope("/users").configure(users::config));
    cfg.service(web::scope("/ws").configure(ws::config));
}

#[derive(Debug, Serialize)]
pub struct SelfInfo {
    pub version: String,
}

async fn self_route() -> ApiResult {
    Ok(HttpResponse::Ok().json(SelfInfo {
        version: std::env!("PLATZ_BACKEND_VERSION").to_string(),
    }))
}
