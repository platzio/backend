use crate::{access_token::AccessToken, error::AuthError};
use platz_db::{
    Identity,
    schema::{bot::Bot, deployment::Deployment, user::User},
};
use serde::Serialize;
use std::borrow::Borrow;

#[derive(Serialize)]
pub struct ApiIdentity(Identity);

impl ApiIdentity {
    pub fn inner(&self) -> &Identity {
        &self.0
    }

    pub fn into_inner(self) -> Identity {
        self.0
    }

    /// Build and validate an identity from a raw access-token (JWT) string.
    /// Used by the websocket authentication path, where the token arrives via
    /// the `Sec-WebSocket-Protocol` header rather than `Authorization`.
    pub async fn from_access_token(token: &str) -> Result<Self, AuthError> {
        let claims = AccessToken::decode(token).await?;
        Self::from(Identity::from(claims)).validate().await
    }

    /// Verify the identity still refers to an existing, active subject.
    pub(crate) async fn validate(self) -> Result<Self, AuthError> {
        match self.inner() {
            Identity::User(user_id) => User::find_only_active(user_id.to_owned())
                .await?
                .map(|_| self)
                .ok_or(AuthError::UserNotFound),
            Identity::Bot(bot_id) => Bot::find(bot_id.to_owned())
                .await?
                .map(|_| self)
                .ok_or(AuthError::BotNotFound),
            Identity::Deployment(deployment_id) => {
                Deployment::find_optional(deployment_id.to_owned())
                    .await?
                    .map(|_| self)
                    .ok_or(AuthError::DeploymentNotFound)
            }
        }
    }
}

impl From<Identity> for ApiIdentity {
    fn from(identity: Identity) -> Self {
        Self(identity)
    }
}

impl From<ApiIdentity> for Identity {
    fn from(api_identity: ApiIdentity) -> Self {
        api_identity.into_inner()
    }
}

impl Borrow<Identity> for ApiIdentity {
    fn borrow(&self) -> &Identity {
        self.inner()
    }
}
