use platz_db::DbError;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] DbError),

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

    #[error("Deployment not found")]
    DeploymentNotFound,

    #[error("JWT decode error")]
    JwtSecretDecodingError,
}
