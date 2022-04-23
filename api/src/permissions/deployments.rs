use super::verify_env_admin;
use crate::result::ApiError;
use platz_db::{Deployment, DeploymentPermission, Identity, K8sCluster};
use uuid::Uuid;

pub async fn verify_deployment_owner<I>(
    cluster_id: Uuid,
    kind: &str,
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
                match DeploymentPermission::find_user_role(env_id, user_id, kind.to_owned()).await?
                {
                    Some(platz_db::UserDeploymentRole::Owner) => Ok(()),
                    _ => Err(ApiError::NoPermission),
                }
            }
        },
        Err(err) => Err(err),
    }
}

pub async fn verify_deployment_maintainer<I>(
    cluster_id: Uuid,
    kind: &str,
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
                match DeploymentPermission::find_user_role(
                    env_id,
                    user_id.to_owned(),
                    kind.to_owned(),
                )
                .await?
                {
                    None => Err(ApiError::NoPermission),
                    Some(_) => Ok(()),
                }
            }
            Identity::Deployment(deployment_id) => {
                let identity_deployment = Deployment::find(deployment_id.to_owned()).await?;
                let identity_env_id = K8sCluster::find(identity_deployment.cluster_id)
                    .await?
                    .env_id;
                if Some(env_id) == identity_env_id {
                    Ok(())
                } else {
                    Err(ApiError::NoPermission)
                }
            }
        },
        Err(err) => Err(err),
    }
}
