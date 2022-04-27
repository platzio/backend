use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use platz_auth::AuthError;
use platz_db::DbError;

#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("{0}")]
    AuthError(#[from] AuthError),

    #[error("{0}")]
    DbError(#[from] DbError),

    #[error("You don't have permissions to perform this operation")]
    NoPermission,
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::AuthError(_) => StatusCode::UNAUTHORIZED,
            Self::DbError(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::NoPermission => StatusCode::FORBIDDEN,
        }
    }
}

pub type ApiResult = Result<HttpResponse, ApiError>;
