use super::verify_site_admin;
use crate::result::ApiError;
use platz_db::{EnvUserPermission, EnvUserRole, Identity};
use uuid::Uuid;

pub async fn verify_env_admin<I>(env_id: Uuid, identity: &I) -> Result<(), ApiError>
where
    I: std::borrow::Borrow<Identity>,
{
    match identity.borrow().user_id() {
        None => Err(ApiError::NoPermission),
        Some(user_id) => match EnvUserPermission::find_user_role_in_env(env_id, user_id).await? {
            Some(EnvUserRole::Admin) => Ok(()),
            _ => verify_site_admin(identity).await,
        },
    }
}
