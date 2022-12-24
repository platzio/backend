use crate::error::AuthError;
use chrono::prelude::*;
use chrono::Duration;
use jsonwebtoken::{encode, EncodingKey, Header};
use platz_db::{Deployment, Identity, Setting, User};
use rand::random;
use serde::{Deserialize, Serialize};

const JWT_SECRET_BYTES: usize = 24;

lazy_static::lazy_static! {
    pub static ref USER_TOKEN_DURATION: Duration = Duration::days(7);
    pub static ref DEPLOYMENT_TOKEN_DURATION: Duration = Duration::hours(1);
}

pub(crate) async fn get_jwt_secret() -> Result<Vec<u8>, AuthError> {
    base64::decode(
        Setting::get_or_set_default("jwt_secret", || {
            base64::encode(random::<[u8; JWT_SECRET_BYTES]>())
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

    pub fn expires_at(&self) -> Result<DateTime<Utc>, AuthError> {
        let naive = NaiveDateTime::from_timestamp_opt(self.exp as i64, 0)
            .ok_or_else(|| AuthError::NaiveDateTimeConvertOverflow(self.exp))?;
        Ok(DateTime::from_utc(naive, Utc))
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

impl From<&Deployment> for AccessToken {
    fn from(deployment: &Deployment) -> Self {
        let iat = chrono::Utc::now();
        let exp = iat + *DEPLOYMENT_TOKEN_DURATION;
        Self {
            iat: iat.timestamp() as usize,
            nbf: iat.timestamp() as usize,
            exp: exp.timestamp() as usize,
            identity: deployment.into(),
        }
    }
}

impl From<AccessToken> for Identity {
    fn from(access_token: AccessToken) -> Self {
        access_token.identity
    }
}
