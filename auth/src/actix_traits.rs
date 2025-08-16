use crate::{
    API_TOKEN_HEADER,
    access_token::{AccessToken, get_jwt_secret},
    api_token::validate_api_token,
    error::AuthError,
};
use actix_web::{FromRequest, HttpRequest, dev::Payload, http::header::HeaderName};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use futures::future::{BoxFuture, FutureExt, TryFutureExt, ok, ready};
use jsonwebtoken::{Algorithm, DecodingKey, TokenData, Validation, decode};
use platz_db::{
    Identity,
    schema::{bot::Bot, deployment::Deployment, user::User},
};

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

impl FromRequest for super::ApiIdentity {
    type Error = AuthError;
    type Future = BoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let headers = req.headers();
        if let Some(user_token_value) = headers.get(HeaderName::from_static(API_TOKEN_HEADER)) {
            // Get (& verify) ApiIdentity from API token header
            ready(
                user_token_value
                    .to_str()
                    .map(ToOwned::to_owned)
                    .map_err(|_| {
                        AuthError::ApiTokenAuthenticationError("API token header has no value")
                    }),
            )
            .and_then(validate_api_token)
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
            AuthError::BotNotFound => actix_web::error::ErrorUnauthorized(reason),
            AuthError::DeploymentNotFound => actix_web::error::ErrorUnauthorized(reason),
            AuthError::JwtSecretDecodingError => actix_web::error::ErrorInternalServerError(reason),
            AuthError::ApiTokenAuthenticationError(_) => {
                actix_web::error::ErrorUnauthorized(reason)
            }
            AuthError::NaiveDateTimeConvertOverflow(_) => {
                actix_web::error::ErrorUnauthorized(reason)
            }
        }
    }
}
