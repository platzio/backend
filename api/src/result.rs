use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use platz_auth::AuthError;
use platz_db::DbError;

#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error(transparent)]
    AuthError(#[from] AuthError),

    #[error(transparent)]
    DbError(DbError),

    #[error("Not found")]
    NotFound,

    #[error("You don't have permissions to perform this operation")]
    NoPermission,
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::AuthError(_) => StatusCode::UNAUTHORIZED,
            Self::DbError(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::NoPermission => StatusCode::FORBIDDEN,
        }
    }
}

pub type ApiResult = Result<HttpResponse, ApiError>;

impl From<DbError> for ApiError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::NotFound => Self::NotFound,
            err => Self::DbError(err),
        }
    }
}
