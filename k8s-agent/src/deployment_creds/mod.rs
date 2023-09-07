use crate::config::OWN_URL;
use crate::k8s::K8S_TRACKER;
use crate::task_runner::apply_secret;
use anyhow::Result;
use futures::future::try_join_all;
use log::*;
use maplit::btreemap;
use platz_auth::{AccessToken, DEPLOYMENT_TOKEN_DURATION};
use platz_db::Deployment;
use tokio::select;
use tokio::time;
use tokio::time::interval;

const CREDS_SECRET_NAME: &str = "platz-creds";
const REFRESH_CREDS_CHUNK_SIZE: usize = 10;
const REFRESH_CREDS_SLEEP_BETWEEN_CHUNKS: time::Duration = time::Duration::from_secs(1);

#[tracing::instrument(err, name = "d-creds")]
pub async fn start() -> Result<()> {
    debug!("starting");
    let refresh_every = *DEPLOYMENT_TOKEN_DURATION / 2;
    let mut interval = interval(refresh_every.to_std()?);
    let mut k8s_events_rx = K8S_TRACKER.outbound_notifications_rx().await;

    loop {
        select! {
            _ = interval.tick() => {
                debug!("interval");
            }
            k8s_event = k8s_events_rx.changed() => {
                tracing::debug!(?k8s_event);
                k8s_event?;
            }
        }

        if let Err(err) = refresh_credentials().await {
            error!("Error refreshing credentials: {:?}", err);
        }
    }
}

#[tracing::instrument(err, name = "refresh")]
async fn refresh_credentials() -> Result<()> {
    debug!("started");

    let cluster_ids = K8S_TRACKER.get_ids().await;

    for deploy_chunk in Deployment::find_by_cluster_ids(cluster_ids)
        .await?
        .chunks(REFRESH_CREDS_CHUNK_SIZE)
    {
        try_join_all(
            deploy_chunk
                .iter()
                .filter(|deployment| deployment.enabled)
                .map(apply_deployment_credentials),
        )
        .await?;
        time::sleep(REFRESH_CREDS_SLEEP_BETWEEN_CHUNKS).await;
    }

    Ok(())
}

#[tracing::instrument(err, fields(deployment=%deployment.id), name="apply-d-creds")]
pub(crate) async fn apply_deployment_credentials(deployment: &Deployment) -> Result<()> {
    debug!("applying");
    let access_token = AccessToken::from(deployment);
    apply_secret(
        deployment.cluster_id,
        &deployment.namespace_name(),
        CREDS_SECRET_NAME,
        btreemap! {
            "access_token".to_owned() => access_token.encode().await?,
            "server_url".to_owned() => OWN_URL.to_string(),
            "expires_at".to_owned() => access_token.expires_at()?.to_rfc3339(),
        },
    )
    .await
}
