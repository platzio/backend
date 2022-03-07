use super::verify_env_admin;
use crate::result::ApiError;
use platz_db::{DeploymentPermission, K8sCluster};
use uuid::Uuid;

pub async fn verify_deployment_owner(
    cluster_id: Uuid,
    kind: &str,
    user_id: Uuid,
) -> Result<(), ApiError> {
    let env_id = K8sCluster::find(cluster_id)
        .await?
        .env_id
        .ok_or(ApiError::NoPermission)?;
    match verify_env_admin(env_id, user_id).await {
        Ok(()) => Ok(()),
        Err(ApiError::NoPermission) => {
            match DeploymentPermission::find_user_role(env_id, user_id, kind.to_owned()).await? {
                Some(platz_db::UserDeploymentRole::Owner) => Ok(()),
                _ => Err(ApiError::NoPermission),
            }
        }
        Err(err) => Err(err),
    }
}

pub async fn verify_deployment_maintainer(
    cluster_id: Uuid,
    kind: &str,
    user_id: Uuid,
) -> Result<(), ApiError> {
    let env_id = K8sCluster::find(cluster_id)
        .await?
        .env_id
        .ok_or(ApiError::NoPermission)?;
    match verify_env_admin(env_id, user_id).await {
        Ok(()) => Ok(()),
        Err(ApiError::NoPermission) => {
            match DeploymentPermission::find_user_role(env_id, user_id, kind.to_owned()).await? {
                None => Err(ApiError::NoPermission),
                Some(_) => Ok(()),
            }
        }
        Err(err) => Err(err),
    }
}
