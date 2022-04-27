use crate::task_runner::apply_secret;
use anyhow::Result;
use futures::future::try_join_all;
use log::*;
use maplit::btreemap;
use platz_auth::{AccessToken, DEPLOYMENT_TOKEN_DURATION};
use platz_db::Deployment;
use tokio::time::interval;

const CREDS_SECRET_NAME: &str = "platz-creds";

pub async fn start() -> Result<()> {
    info!("Deployment credentials task starting");
    let refresh_every = *DEPLOYMENT_TOKEN_DURATION / 2;
    let mut interval = interval(refresh_every.to_std()?);

    loop {
        interval.tick().await;
        info!("Refreshing deployment credentials");
        if let Err(err) = refresh_credentials().await {
            error!("Error refreshing credentials: {:?}", err);
        }
    }
}

async fn refresh_credentials() -> Result<()> {
    try_join_all(
        Deployment::all()
            .await?
            .iter()
            .map(apply_deployment_credentials),
    )
    .await?;
    Ok(())
}

pub(crate) async fn apply_deployment_credentials(deployment: &Deployment) -> Result<()> {
    let access_token = AccessToken::from(deployment).encode().await?;
    apply_secret(
        deployment.cluster_id,
        &deployment.namespace_name(),
        CREDS_SECRET_NAME,
        btreemap! {
            "access_token".to_owned() => access_token,
        },
    )
    .await
}
