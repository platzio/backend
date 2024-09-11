mod access_token;
mod api_token;
mod error;
mod identity;
mod oidc;

#[cfg(feature = "actix")]
mod actix_traits;

pub const API_TOKEN_HEADER: &str = "x-platz-token";

pub use access_token::{AccessToken, DEPLOYMENT_TOKEN_DURATION, USER_TOKEN_DURATION};
pub use api_token::generate_api_token;
pub use error::AuthError;
pub use identity::ApiIdentity;
pub use oidc::{Config, OAuth2Response, OidcLogin};
