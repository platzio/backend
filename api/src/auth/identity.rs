use super::{AccessToken, AuthError};
use actix_web::{dev::Payload, FromRequest, HttpRequest};
use futures::future::{ok, BoxFuture, FutureExt, TryFutureExt};
use platz_db::{Deployment, Identity, User};
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

    async fn validate(self) -> Result<Self, AuthError> {
        match self.inner() {
            Identity::User(user_id) => match User::find(user_id.to_owned()).await {
                Err(err) => Err(err.into()),
                Ok(None) => Err(AuthError::UserNotFound),
                Ok(Some(_)) => Ok(self),
            },
            Identity::Deployment(deployment_id) => {
                match Deployment::find_optional(deployment_id.to_owned()).await {
                    Err(err) => Err(err.into()),
                    Ok(None) => Err(AuthError::DeploymentNotFound),
                    Ok(Some(_)) => Ok(self),
                }
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

impl FromRequest for ApiIdentity {
    type Error = AuthError;
    type Future = BoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        AccessToken::from_request(req, payload)
            .and_then(|access_token| ok(Identity::from(access_token)))
            .and_then(|identity| ok(ApiIdentity::from(identity)))
            .and_then(|api_identity| api_identity.validate())
            .boxed()
    }
}
