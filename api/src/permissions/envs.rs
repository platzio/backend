use crate::result::ApiError;
use platz_db::{EnvUserPermission, EnvUserRole};
use uuid::Uuid;

pub async fn verify_env_admin(env_id: Uuid, user_id: Uuid) -> Result<(), ApiError> {
    match EnvUserPermission::find_user_role_in_env(env_id, user_id).await? {
        Some(EnvUserRole::Admin) => Ok(()),
        _ => Err(ApiError::NoPermission),
    }
}
