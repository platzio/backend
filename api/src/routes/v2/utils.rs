use crate::result::ApiError;
use platz_auth::{ApiIdentity, AuthError};
use platz_db::User;
use uuid::Uuid;

pub(super) fn ensure_user_id(identity: &ApiIdentity) -> Result<Uuid, ApiError> {
    identity.inner().user_id().ok_or(ApiError::NoPermission)
}

pub(super) async fn ensure_user(identity: &ApiIdentity) -> Result<User, ApiError> {
    User::find_only_active(ensure_user_id(identity)?)
        .await?
        .ok_or_else(|| {
            ApiError::from(AuthError::BearerAuthenticationError(
                "Unknown user".to_owned(),
            ))
        })
}
