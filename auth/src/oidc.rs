use crate::error::AuthError;
use openid::DiscoveredClient;
use platz_db::{NewUser, User};
use serde::Deserialize;
use url::Url;

pub struct OidcLogin {
    pub server: Url,
    pub client_id: String,
    pub client_secret: String,
    pub admin_emails: Vec<String>,
}

#[derive(Deserialize)]
pub struct OAuth2Response {
    auth_code: String,
}

impl OidcLogin {
    pub async fn new(
        server: Url,
        client_id: String,
        client_secret: String,
        admin_emails: Vec<String>,
    ) -> Result<Self, AuthError> {
        Ok(Self {
            server,
            client_id,
            client_secret,
            admin_emails,
        })
    }

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
