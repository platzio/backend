use crate::access_token::{get_jwt_secret, AccessToken};
use crate::error::AuthError;
use actix_web::{dev::Payload, http::header::HeaderName, FromRequest, HttpRequest};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use futures::future::{ok, BoxFuture, FutureExt, TryFutureExt};
use jsonwebtoken::{decode, Algorithm, DecodingKey, TokenData, Validation};
use platz_db::{Deployment, Identity, User};

async fn validate_token(bearer: BearerAuth) -> Result<TokenData<AccessToken>, AuthError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_required_spec_claims(&["exp", "nbf"]);
    validation.validate_exp = true;
    validation.validate_nbf = true;
    validation.leeway = 5;
    let jwt_secret = get_jwt_secret().await?;
    decode::<AccessToken>(
        bearer.token(),
        &DecodingKey::from_secret(&jwt_secret),
        &validation,
    )
    .map_err(AuthError::JwtDecodeError)
}

impl FromRequest for AccessToken {
    type Error = AuthError;
    type Future = BoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        BearerAuth::from_request(req, payload)
            .map_err(|e| AuthError::BearerAuthenticationError(e.to_string()))
            .and_then(validate_token)
            .map_ok(|token_data| token_data.claims)
            .boxed()
    }
}

impl super::ApiIdentity {
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

impl FromRequest for super::ApiIdentity {
    type Error = AuthError;
    type Future = BoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let headers = req.headers();
        if let Some(user_token_value) = headers.get(HeaderName::from_static("x-platz-token")) {
            // Get (& verify) ApiIdentity from UserToken
            let user_token_value_str = user_token_value.to_str().unwrap().to_string();
            crate::user_token::validate_user_token(user_token_value_str)
                .and_then(|user_token| ok(Identity::User(user_token.user_id)))
                .and_then(|identity| ok(Self::from(identity)))
                .and_then(|api_identity| api_identity.validate())
                .boxed()
        } else {
            // Get (& verify) ApiIdentity from AccessToken
            AccessToken::from_request(req, payload)
                .and_then(|access_token| ok(Identity::from(access_token)))
                .and_then(|identity| ok(Self::from(identity)))
                .and_then(|api_identity| api_identity.validate())
                .boxed()
        }
    }
}

impl From<AuthError> for actix_web::Error {
    fn from(err: AuthError) -> Self {
        let reason = err.to_string();
        match err {
            AuthError::DatabaseError(_) => actix_web::error::ErrorServiceUnavailable(reason),
            AuthError::JoinError(_) => actix_web::error::ErrorInternalServerError(reason),
            AuthError::OidcDiscoveryError(_) => actix_web::error::ErrorServiceUnavailable(reason),
            AuthError::OidcLoginError(_) => actix_web::error::ErrorUnauthorized(reason),
            AuthError::OidcResponseError(_) => actix_web::error::ErrorInternalServerError(reason),
            AuthError::BearerAuthenticationError(_) => actix_web::error::ErrorUnauthorized(reason),
            AuthError::JwtEncodeError(_) => actix_web::error::ErrorServiceUnavailable(reason),
            AuthError::JwtDecodeError(_) => actix_web::error::ErrorUnauthorized(reason),
            AuthError::UserNotFound => actix_web::error::ErrorUnauthorized(reason),
            AuthError::DeploymentNotFound => actix_web::error::ErrorUnauthorized(reason),
            AuthError::JwtSecretDecodingError => actix_web::error::ErrorInternalServerError(reason),
            AuthError::UserTokenAuthenticationError(_) => {
                actix_web::error::ErrorUnauthorized(reason)
            }
            AuthError::NaiveDateTimeConvertOverflow(_) => {
                actix_web::error::ErrorUnauthorized(reason)
            }
        }
    }
}
