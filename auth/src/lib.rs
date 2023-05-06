mod access_token;
mod error;
mod identity;
mod oidc;
mod user_token;

#[cfg(feature = "actix")]
mod actix_traits;

pub const USER_TOKEN_HEADER: &str = "x-platz-token";

pub use access_token::{AccessToken, DEPLOYMENT_TOKEN_DURATION, USER_TOKEN_DURATION};
pub use error::AuthError;
pub use identity::ApiIdentity;
pub use oidc::{Config, OAuth2Response, OidcLogin};
pub use user_token::generate_user_token;
