use platz_db::DbError;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] DbError),

    #[error("Error reading SSM parameter {0}: {1}")]
    SsmParamReadError(String, String),

    #[error("OIDC discovery error: {0}")]
    OidcDiscoveryError(openid::error::Error),

    #[error("OIDC authentication error: {0}")]
    OidcLoginError(openid::error::Error),

    #[error("OIDC response error: {0}")]
    OidcResponseError(String),

    #[error("Bearer authentication error: {0}")]
    BearerAuthenticationError(String),

    #[error("JWT encoding error: {0}")]
    JwtEncodeError(jsonwebtoken::errors::Error),

    #[error("JWT decoding error: {0}")]
    JwtDecodeError(jsonwebtoken::errors::Error),

    #[error("User not found")]
    UserNotFound,

    #[error("JWT decode error")]
    JwtSecretDecodingError,
}

impl From<AuthError> for actix_web::Error {
    fn from(err: AuthError) -> Self {
        let reason = err.to_string();
        match err {
            AuthError::DatabaseError(_) => actix_web::error::ErrorServiceUnavailable(reason),
            AuthError::OidcDiscoveryError(_) => actix_web::error::ErrorServiceUnavailable(reason),
            AuthError::OidcLoginError(_) => actix_web::error::ErrorUnauthorized(reason),
            AuthError::OidcResponseError(_) => actix_web::error::ErrorInternalServerError(reason),
            AuthError::SsmParamReadError(_, _) => actix_web::error::ErrorServiceUnavailable(reason),
            AuthError::BearerAuthenticationError(_) => actix_web::error::ErrorUnauthorized(reason),
            AuthError::JwtEncodeError(_) => actix_web::error::ErrorServiceUnavailable(reason),
            AuthError::JwtDecodeError(_) => actix_web::error::ErrorUnauthorized(reason),
            AuthError::UserNotFound => actix_web::error::ErrorUnauthorized(reason),
            AuthError::JwtSecretDecodingError => actix_web::error::ErrorInternalServerError(reason),
        }
    }
}
