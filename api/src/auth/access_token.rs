use super::AuthError;
use actix_web::{dev::Payload, FromRequest, HttpRequest};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use futures::future::{BoxFuture, FutureExt, TryFutureExt};
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use platz_db::{Setting, User};
use rand::random;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const JWT_SECRET_BYTES: usize = 24;

async fn get_jwt_secret() -> Result<Vec<u8>, AuthError> {
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
    sub: Uuid,
}

impl AccessToken {
    pub fn user_id(&self) -> Uuid {
        self.sub
    }

    pub async fn encode(&self) -> Result<String, AuthError> {
        let jwt_secret = get_jwt_secret().await?;
        encode(
            &Header::default(),
            &self,
            &EncodingKey::from_secret(&jwt_secret),
        )
        .map_err(AuthError::JwtEncodeError)
    }
}

impl From<&User> for AccessToken {
    fn from(user: &User) -> Self {
        let iat = chrono::Utc::now();
        let exp = iat + chrono::Duration::days(7);
        Self {
            iat: iat.timestamp() as usize,
            nbf: iat.timestamp() as usize,
            exp: exp.timestamp() as usize,
            sub: user.id,
        }
    }
}

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
