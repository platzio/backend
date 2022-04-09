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
mod k8s_clusters;
mod k8s_resources;
mod secrets;
mod users;
mod ws;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
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
    cfg.service(web::scope("/k8s-clusters").configure(k8s_clusters::config));
    cfg.service(web::scope("/k8s-resources").configure(k8s_resources::config));
    cfg.service(web::scope("/secrets").configure(secrets::config));
    cfg.service(web::scope("/users").configure(users::config));
    cfg.service(web::scope("/ws").configure(ws::config));
}
