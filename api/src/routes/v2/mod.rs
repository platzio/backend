mod auth;
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
mod server;
mod user_tokens;
mod users;
mod ws;

use actix_web::web;
use platz_db::{DbTable, DbTableOrDeploymentResource};
use utoipa::OpenApi;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(auth::me);
    cfg.service(auth::start_google_login);
    cfg.service(auth::finish_google_login);
    cfg.service(deployment_permissions::get_all);
    cfg.service(deployment_permissions::get_one);
    cfg.service(deployment_permissions::create);
    cfg.service(deployment_permissions::delete);
    cfg.service(deployment_resource_types::get_all);
    cfg.service(deployment_resource_types::get_one);
    cfg.service(deployment_resources::get_all);
    cfg.service(deployment_resources::get_one);
    cfg.service(deployment_resources::create);
    cfg.service(deployment_resources::update);
    cfg.service(deployment_resources::delete);
    cfg.service(deployment_tasks::get_all);
    cfg.service(deployment_tasks::get_one);
    cfg.service(deployment_tasks::create);
    cfg.service(deployments::get_all);
    cfg.service(deployments::get_one);
    cfg.service(deployments::create);
    cfg.service(deployments::update);
    cfg.service(deployments::delete);
    cfg.service(env_user_permissions::get_all);
    cfg.service(env_user_permissions::get_one);
    cfg.service(env_user_permissions::create);
    cfg.service(env_user_permissions::delete);
    cfg.service(envs::get_all);
    cfg.service(envs::get_one);
    cfg.service(envs::create);
    cfg.service(envs::update);
    cfg.service(envs::delete);
    cfg.service(helm_charts::get_all);
    cfg.service(helm_charts::get_one);
    cfg.service(helm_registries::get_all);
    cfg.service(helm_registries::get_one);
    cfg.service(helm_registries::update);
    cfg.service(helm_tag_formats::get_all);
    cfg.service(helm_tag_formats::get_one);
    cfg.service(helm_tag_formats::create);
    cfg.service(helm_tag_formats::delete);
    cfg.service(k8s_clusters::get_all);
    cfg.service(k8s_resources::get_all);
    cfg.service(k8s_clusters::update);
    cfg.service(k8s_clusters::delete);
    cfg.service(k8s_resources::get_one);
    cfg.service(secrets::get_all);
    cfg.service(secrets::get_one);
    cfg.service(secrets::create);
    cfg.service(secrets::update);
    cfg.service(secrets::delete);
    cfg.service(server::get_one);
    cfg.service(user_tokens::get_all);
    cfg.service(user_tokens::get_one);
    cfg.service(user_tokens::create);
    cfg.service(user_tokens::delete);
    cfg.service(users::get_all);
    cfg.service(users::get_one);
    cfg.service(users::update);
    cfg.service(web::scope("/ws").configure(ws::config));
}

#[derive(OpenApi)]
#[openapi(components(schemas(DbTable, DbTableOrDeploymentResource)))]
pub(super) struct ApiV2;

impl ApiV2 {
    pub fn openapi() -> utoipa::openapi::OpenApi {
        let mut openapi = <ApiV2 as OpenApi>::openapi();
        openapi.merge(auth::OpenApi::openapi());
        openapi.merge(platz_chart_ext::openapi::OpenApi::openapi());
        openapi.merge(deployment_permissions::OpenApi::openapi());
        openapi.merge(deployment_resource_types::OpenApi::openapi());
        openapi.merge(deployment_resources::OpenApi::openapi());
        openapi.merge(deployment_tasks::OpenApi::openapi());
        openapi.merge(deployments::OpenApi::openapi());
        openapi.merge(env_user_permissions::OpenApi::openapi());
        openapi.merge(envs::OpenApi::openapi());
        openapi.merge(helm_charts::OpenApi::openapi());
        openapi.merge(helm_registries::OpenApi::openapi());
        openapi.merge(helm_tag_formats::OpenApi::openapi());
        openapi.merge(k8s_clusters::OpenApi::openapi());
        openapi.merge(k8s_resources::OpenApi::openapi());
        openapi.merge(secrets::OpenApi::openapi());
        openapi.merge(server::OpenApi::openapi());
        openapi.merge(user_tokens::OpenApi::openapi());
        openapi.merge(users::OpenApi::openapi());
        openapi
    }
}
