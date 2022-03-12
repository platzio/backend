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
mod k8s_clusters;
mod k8s_resources;
mod secrets;
mod status;
mod users;
mod ws;

pub fn config(cfg: &mut actix_web::web::ServiceConfig) {
    auth::config(cfg);
    deployments::config(cfg);
    deployment_permissions::config(cfg);
    deployment_resource_types::config(cfg);
    deployment_resources::config(cfg);
    deployment_tasks::config(cfg);
    envs::config(cfg);
    env_user_permissions::config(cfg);
    helm_charts::config(cfg);
    helm_registries::config(cfg);
    k8s_clusters::config(cfg);
    k8s_resources::config(cfg);
    secrets::config(cfg);
    status::config(cfg);
    users::config(cfg);
    ws::config(cfg);
}
