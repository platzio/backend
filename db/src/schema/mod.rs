mod deployment;
mod deployment_kind;
mod deployment_permission;
mod deployment_resource;
mod deployment_resource_type;
mod deployment_status;
mod deployment_task;
mod env;
mod env_user_permission;
mod helm_chart;
mod helm_registry;
mod helm_tag_format;
mod k8s_cluster;
mod k8s_resource;
mod secret;
mod setting;
mod user;
mod user_token;

pub use deployment::*;
pub use deployment_kind::*;
pub use deployment_permission::*;
pub use deployment_resource::*;
pub use deployment_resource_type::*;
pub use deployment_status::*;
pub use deployment_task::*;
pub use env::*;
pub use env_user_permission::*;
pub use helm_chart::*;
pub use helm_registry::*;
pub use helm_tag_format::*;
pub use k8s_cluster::*;
pub use k8s_resource::*;
pub use secret::*;
pub use setting::*;
pub use user::*;
pub use user_token::*;

diesel::allow_tables_to_appear_in_same_query!(
    deployments,
    deployment_kinds,
    deployment_permissions,
    deployment_resources,
    deployment_resource_types,
    deployment_tasks,
    envs,
    env_user_permissions,
    helm_charts,
    helm_registries,
    helm_tag_formats,
    k8s_clusters,
    k8s_resources,
    secrets,
    settings,
    users,
    user_tokens,
);
