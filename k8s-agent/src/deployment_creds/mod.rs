use crate::{config::Config, k8s::tracker::K8S_TRACKER, task_runner::apply_secret};
use anyhow::Result;
use futures::future::try_join_all;
use maplit::btreemap;
use platz_auth::{AccessToken, DEPLOYMENT_TOKEN_DURATION};
use platz_db::schema::deployment::Deployment;
use tokio::{
    select,
    time::{self, interval},
};
use tracing::{debug, error};
use url::Url;

const CREDS_SECRET_NAME: &str = "platz-creds";
const REFRESH_CREDS_CHUNK_SIZE: usize = 10;
const REFRESH_CREDS_SLEEP_BETWEEN_CHUNKS: time::Duration = time::Duration::from_secs(1);

#[tracing::instrument(err, skip_all, name = "d-creds")]
pub async fn start(config: &Config) -> Result<()> {
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

        if config.should_refresh_deployment_credintials() {
            if let Err(err) = refresh_credentials(config).await {
                error!("Error refreshing credentials: {:?}", err);
            }
        }
    }
}

#[tracing::instrument(err, skip_all, name = "refresh")]
async fn refresh_credentials(config: &Config) -> Result<()> {
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
                .map(|deployment| apply_deployment_credentials(deployment, &config.platz_url)),
        )
        .await?;
        time::sleep(REFRESH_CREDS_SLEEP_BETWEEN_CHUNKS).await;
    }

    Ok(())
}

#[tracing::instrument(err, fields(deployment=%deployment.id), name="apply-d-creds")]
pub(crate) async fn apply_deployment_credentials(
    deployment: &Deployment,
    platz_url: &Url,
) -> Result<()> {
    debug!("applying");
    let access_token = AccessToken::from(deployment);
    apply_secret(
        deployment.cluster_id,
        &deployment.namespace_name().await,
        CREDS_SECRET_NAME,
        btreemap! {
            "access_token".to_owned() => access_token.encode().await?,
            "server_url".to_owned() => platz_url.to_string(),
            "expires_at".to_owned() => access_token.expires_at()?.to_rfc3339(),
        },
    )
    .await
}
