use anyhow::Result;
use async_trait::async_trait;
use log::*;
use rusoto_core::{Client, HttpClient};
use rusoto_credential::{
    AutoRefreshingProvider, AwsCredentials, CredentialsError, DefaultCredentialsProvider,
    ProvideAwsCredentials,
};
use rusoto_sts::{StsAssumeRoleSessionCredentialsProvider, StsClient, WebIdentityProvider};
use std::env;

/// Create a rusoto_core::Client with a credentials provider that
/// matches the current environment (K8S first, then trying to assume
/// role, then using the default environment/files/etc.)
///
/// The client has to be used with the `new_with_client` method of
/// other rusoto objects (e.g. S3Client::new_with_client)

pub fn rusoto_client(session_name: String) -> Result<Client> {
    let provider = rusoto_credentials_provider(session_name)?;
    let request_dispatcher = HttpClient::new()?;
    Ok(Client::new_with(provider, request_dispatcher))
}

pub enum CustomCredentialsProvider {
    K8s(Box<AutoRefreshingProvider<WebIdentityProvider>>),
    Role(Box<AutoRefreshingProvider<StsAssumeRoleSessionCredentialsProvider>>),
    Default(Box<DefaultCredentialsProvider>),
}

#[async_trait]
impl ProvideAwsCredentials for CustomCredentialsProvider {
    async fn credentials(&self) -> Result<AwsCredentials, CredentialsError> {
        match self {
            Self::K8s(p) => p.credentials().await,
            Self::Role(p) => p.credentials().await,
            Self::Default(p) => p.credentials().await,
        }
    }
}

pub fn rusoto_credentials_provider(session_name: String) -> Result<CustomCredentialsProvider> {
    if env::var("AWS_WEB_IDENTITY_TOKEN_FILE").is_ok() {
        debug!("Found AWS_WEB_IDENTITY_TOKEN_FILE environment variable, using K8S credentials");
        let provider = WebIdentityProvider::from_k8s_env();
        return Ok(CustomCredentialsProvider::K8s(Box::new(
            AutoRefreshingProvider::new(provider)?,
        )));
    }

    if let Ok(role_arn) = env::var("AWS_ROLE_ARN") {
        debug!("Found AWS_ROLE_ARN environment variable, assuming role");
        let sts = StsClient::new(Default::default());
        let provider = StsAssumeRoleSessionCredentialsProvider::new(
            sts,
            role_arn,
            session_name,
            None,
            None,
            None,
            None,
        );
        return Ok(CustomCredentialsProvider::Role(Box::new(
            AutoRefreshingProvider::new(provider)?,
        )));
    }

    Ok(CustomCredentialsProvider::Default(Box::new(
        DefaultCredentialsProvider::new()?,
    )))
}
