use super::verify_env_admin;
use crate::result::ApiError;
use platz_db::{
    Identity,
    schema::{
        bot::Bot,
        deployment::Deployment,
        deployment_permission::{DeploymentPermission, UserDeploymentRole},
        k8s_cluster::K8sCluster,
    },
};
use uuid::Uuid;

pub async fn verify_deployment_owner<I>(
    cluster_id: Uuid,
    kind_id: Uuid,
    identity: &I,
) -> Result<(), ApiError>
where
    I: std::borrow::Borrow<Identity>,
{
    let env_id = K8sCluster::find(cluster_id)
        .await?
        .env_id
        .ok_or(ApiError::NoPermission)?;
    match verify_env_admin(env_id, identity).await {
        Ok(()) => Ok(()),
        Err(ApiError::NoPermission) => match identity.borrow().user_id() {
            None => Err(ApiError::NoPermission),
            Some(user_id) => {
                match DeploymentPermission::find_user_role(env_id, user_id, kind_id).await? {
                    Some(UserDeploymentRole::Owner) => Ok(()),
                    _ => Err(ApiError::NoPermission),
                }
            }
        },
        Err(err) => Err(err),
    }
}

pub async fn verify_deployment_maintainer<I>(
    cluster_id: Uuid,
    kind_id: Uuid,
    identity: &I,
) -> Result<(), ApiError>
where
    I: std::borrow::Borrow<Identity>,
{
    let env_id = K8sCluster::find(cluster_id)
        .await?
        .env_id
        .ok_or(ApiError::NoPermission)?;
    match verify_env_admin(env_id, identity).await {
        Ok(()) => Ok(()),
        Err(ApiError::NoPermission) => match identity.borrow() {
            Identity::User(user_id) => {
                DeploymentPermission::find_user_role(env_id, user_id.to_owned(), kind_id)
                    .await?
                    .map(|_| ())
                    .ok_or(ApiError::NoPermission)
            }
            Identity::Bot(bot_id) => {
                let _bot = Bot::find(bot_id.to_owned()).await?;
                // TODO: Add bot permissions
                Ok(())
            }
            Identity::Deployment(deployment_id) => {
                let identity_deployment = Deployment::find(deployment_id.to_owned()).await?;
                K8sCluster::find(identity_deployment.cluster_id)
                    .await?
                    .env_id
                    .map(|_| ())
                    .ok_or(ApiError::NoPermission)
            }
        },
        Err(err) => Err(err),
    }
}
