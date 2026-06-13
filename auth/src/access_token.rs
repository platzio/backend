use crate::error::AuthError;
use base64::prelude::*;
use chrono::Duration;
use chrono::prelude::*;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use platz_db::{
    Identity,
    schema::{deployment::Deployment, setting::Setting, user::User},
};
use rand::random;
use serde::{Deserialize, Serialize};

const JWT_SECRET_BYTES: usize = 24;

lazy_static::lazy_static! {
    pub static ref USER_TOKEN_DURATION: Duration = Duration::days(7);
}

pub(crate) async fn get_jwt_secret() -> Result<Vec<u8>, AuthError> {
    BASE64_STANDARD
        .decode(
            Setting::get_or_set_default("jwt_secret", || {
                BASE64_STANDARD.encode(random::<[u8; JWT_SECRET_BYTES]>())
            })
            .await?
            .value
            .as_str(),
        )
        .map_err(|_| AuthError::JwtSecretDecodingError)
}

#[derive(Serialize, Deserialize)]
pub struct AccessToken {
    iat: usize,
    exp: usize,
    nbf: usize,
    identity: Identity,
}

impl AccessToken {
    pub async fn encode(&self) -> Result<String, AuthError> {
        let jwt_secret = get_jwt_secret().await?;
        encode(
            &Header::default(),
            &self,
            &EncodingKey::from_secret(&jwt_secret),
        )
        .map_err(AuthError::JwtEncodeError)
    }

    /// Decode and validate a signed access token (JWT) string into its claims.
    /// Shared by the `Authorization: Bearer` extractor and the websocket
    /// subprotocol authentication path, which cannot use request headers.
    pub(crate) async fn decode(token: &str) -> Result<Self, AuthError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_required_spec_claims(&["exp", "nbf"]);
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.leeway = 5;
        let jwt_secret = get_jwt_secret().await?;
        Ok(
            decode::<Self>(token, &DecodingKey::from_secret(&jwt_secret), &validation)
                .map_err(AuthError::JwtDecodeError)?
                .claims,
        )
    }

    pub fn expires_at(&self) -> Result<DateTime<Utc>, AuthError> {
        DateTime::from_timestamp(self.exp as i64, 0)
            .ok_or_else(|| AuthError::NaiveDateTimeConvertOverflow(self.exp))
    }

    pub fn for_deployment(deployment: &Deployment, duration: Duration) -> Self {
        let iat = chrono::Utc::now();
        let exp = iat + duration;
        Self {
            iat: iat.timestamp() as usize,
            nbf: iat.timestamp() as usize,
            exp: exp.timestamp() as usize,
            identity: deployment.into(),
        }
    }
}

impl From<&User> for AccessToken {
    fn from(user: &User) -> Self {
        let iat = chrono::Utc::now();
        let exp = iat + *USER_TOKEN_DURATION;
        Self {
            iat: iat.timestamp() as usize,
            nbf: iat.timestamp() as usize,
            exp: exp.timestamp() as usize,
            identity: user.into(),
        }
    }
}

impl From<AccessToken> for Identity {
    fn from(access_token: AccessToken) -> Self {
        access_token.identity
    }
}
