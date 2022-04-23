use crate::result::ApiError;
use platz_db::{Identity, User};

pub async fn verify_site_admin<I>(identity: &I) -> Result<(), ApiError>
where
    I: std::borrow::Borrow<Identity>,
{
    match identity.borrow().user_id() {
        None => Err(ApiError::NoPermission),
        Some(user_id) => {
            match User::find(user_id)
                .await?
                .ok_or(ApiError::NoPermission)?
                .is_admin
            {
                true => Ok(()),
                false => Err(ApiError::NoPermission),
            }
        }
    }
}
