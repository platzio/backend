use crate::error::AuthError;
use openid::DiscoveredClient;
use platz_db::{NewUser, User};
use serde::Deserialize;
use url::Url;
use utoipa::ToSchema;

#[derive(clap::Args)]
#[group(skip)]
pub struct Config {
    #[arg(long, env = "OIDC_SERVER_URL")]
    oidc_server_url: Url,

    #[arg(long, env = "OIDC_CLIENT_ID")]
    oidc_client_id: String,

    #[arg(long, env = "OIDC_CLIENT_SECRET", hide_env_values = true)]
    oidc_client_secret: String,

    /// Email addresses to add as admins instead of regular user. This option
    /// is useful for allowing the first admins to log into Platz on a fresh
    /// deployment. Note that admins are added only after successful validation
    /// against the OIDC server, and if a user doesn't exist with that email.
    /// This means that if an admin is later changed to a regular user role,
    /// they will never become an admin again unless their user is deleted from
    /// the database, or removed from this option.
    #[arg(long = "admin-email", env = "ADMIN_EMAILS", value_delimiter = ' ')]
    admin_emails: Vec<String>,
}

impl From<Config> for OidcLogin {
    fn from(config: Config) -> Self {
        Self {
            server: config.oidc_server_url,
            client_id: config.oidc_client_id,
            client_secret: config.oidc_client_secret,
            admin_emails: config.admin_emails,
        }
    }
}

pub struct OidcLogin {
    pub server: Url,
    pub client_id: String,
    pub client_secret: String,
    pub admin_emails: Vec<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct OAuth2Response {
    auth_code: String,
}

impl OidcLogin {
    async fn client(&self, callback_url: &Url) -> Result<DiscoveredClient, AuthError> {
        DiscoveredClient::discover(
            self.client_id.clone(),
            self.client_secret.clone(),
            Some(callback_url.to_string()),
            self.server.clone(),
        )
        .await
        .map_err(AuthError::OidcDiscoveryError)
    }

    pub async fn get_redirect_url(&self, callback_url: &Url) -> Result<Url, AuthError> {
        let client = self.client(callback_url).await?;

        let options = openid::Options {
            scope: Some("openid profile email".to_owned()),
            ..Default::default()
        };

        Ok(client.auth_url(&options))
    }

    async fn validate_user(
        &self,
        callback_url: &Url,
        oauth2_response: OAuth2Response,
    ) -> Result<NewUser, AuthError> {
        let client = self.client(callback_url).await?;
        let token = client
            .authenticate(&oauth2_response.auth_code, None, None)
            .await
            .map_err(AuthError::OidcLoginError)?;
        let userinfo = client
            .request_userinfo(&token)
            .await
            .map_err(AuthError::OidcLoginError)?;
        let display_name = userinfo.name.ok_or_else(|| {
            AuthError::OidcResponseError("Login succeeded but user has no name".to_owned())
        })?;
        let email = userinfo.email.ok_or_else(|| {
            AuthError::OidcResponseError("Login succeeded but user has no email address".to_owned())
        })?;
        let is_admin = self.admin_emails.contains(&email);
        Ok(NewUser {
            display_name,
            email,
            is_admin,
        })
    }

    pub async fn login_user(
        &self,
        callback_url: &Url,
        oauth2_response: OAuth2Response,
    ) -> Result<User, AuthError> {
        let new_user = self.validate_user(callback_url, oauth2_response).await?;
        Ok(match User::find_by_email(&new_user.email).await? {
            Some(user) => user,
            None => new_user.insert().await?,
        })
    }
}
