use crate::result::ApiError;
use platz_db::User;
use uuid::Uuid;

pub async fn verify_site_admin(user_id: Uuid) -> Result<(), ApiError> {
    match User::find(user_id)
        .await?
        .ok_or(ApiError::NoPermission)?
        .is_admin
    {
        true => Ok(()),
        false => Err(ApiError::NoPermission),
    }
}
