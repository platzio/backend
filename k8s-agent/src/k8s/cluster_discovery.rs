use super::{
    cluster_type::{K8s, LocalCluster},
    tracker::K8S_TRACKER,
};
use anyhow::{Result, anyhow};
use aws_types::region::Region;
use clap::ValueEnum;
use futures::future::try_join_all;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::time;
use tracing::{debug, error, info};

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
#[clap(rename_all = "lowercase")]
pub enum ClusterProvider {
    /// Discover clusters by scanning AWS EKS in every region of the running account.
    Eks,
    /// Register a single cluster from a kubeconfig file.
    Local,
}

#[derive(clap::Args)]
#[group(skip)]
pub struct Config {
    #[arg(long, env = "K8S_REFRESH_INTERVAL", default_value = "1h")]
    pub k8s_refresh_interval: humantime::Duration,

    /// Selects how clusters are discovered. Defaults to `eks` (production behaviour);
    /// set to `local` for laptop/dev workflows that target a kubeconfig context.
    #[arg(long, env = "PLATZ_CLUSTER_PROVIDER", value_enum, default_value = "eks")]
    pub provider: ClusterProvider,

    /// Path to the kubeconfig file used in `local` mode.
    /// Falls back to `$KUBECONFIG`, then `~/.kube/config`.
    #[arg(long, env = "PLATZ_LOCAL_KUBECONFIG")]
    pub local_kubeconfig: Option<PathBuf>,

    /// Name of the kubeconfig context to register in `local` mode.
    /// Defaults to the kubeconfig's `current-context`.
    #[arg(long, env = "PLATZ_LOCAL_CONTEXT")]
    pub local_context: Option<String>,
}

pub async fn run_cluster_discovery(config: &Config) -> Result<()> {
    let mut interval = time::interval(config.k8s_refresh_interval.into());

    loop {
        interval.tick().await;
        if let Err(err) = load_clusters(config).await {
            error!("Error scanning for clusters: {:?}", err);
        }
    }
}

async fn load_clusters(config: &Config) -> Result<()> {
    let tracker_tx = K8S_TRACKER.inbound_requests_tx().await;

    for cluster in discover_clusters(config).await?.into_iter() {
        tracing::debug!(%cluster);
        tracker_tx.send(Arc::new(cluster))?;
    }

    Ok(())
}

#[tracing::instrument(skip_all, err, ret)]
async fn discover_clusters(config: &Config) -> Result<Vec<K8s>> {
    match config.provider {
        ClusterProvider::Eks => discover_eks_clusters().await,
        ClusterProvider::Local => discover_local_cluster(config).await.map(|c| vec![c]),
    }
}

async fn discover_eks_clusters() -> Result<Vec<K8s>> {
    debug!("starting EKS discovery...");
    let shared_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let ec2 = aws_sdk_ec2::Client::new(&shared_config);
    debug!("discovering regions...");
    let regions = ec2
        .describe_regions()
        .send()
        .await?
        .regions
        .ok_or_else(|| anyhow!("Got an empty region list"))?
        .into_iter()
        .filter_map(|ec2_region| ec2_region.region_name().map(ToOwned::to_owned))
        .map(Region::new);

    debug!("discovering...");
    let results = try_join_all(regions.map(get_clusters)).await?;
    Ok(results.into_iter().flatten().collect())
}

#[tracing::instrument(err, fields(region=%region))]
async fn get_clusters(region: Region) -> Result<Vec<K8s>> {
    debug!("started");
    let shared_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let client_config = aws_sdk_eks::config::Builder::from(&shared_config)
        .region(Some(region.clone()))
        .build();
    let eks = aws_sdk_eks::Client::from_conf(client_config);

    let cluster_names = eks
        .list_clusters()
        .send()
        .await?
        .clusters
        .ok_or_else(|| anyhow!("Got empty clusters from list_clusters"))?;

    let eks_clusters = try_join_all(cluster_names.into_iter().map(|name| {
        tracing::debug!(name, "describing");
        eks.describe_cluster().name(name).send()
    }))
    .await?
    .into_iter()
    .map(|res| {
        res.cluster
            .ok_or_else(|| anyhow!("describe_cluster returned empty cluster"))
    })
    .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(eks_clusters.into_iter().map(K8s::from).collect())
}

#[tracing::instrument(skip_all, err)]
async fn discover_local_cluster(config: &Config) -> Result<K8s> {
    let kubeconfig = load_local_kubeconfig(config.local_kubeconfig.as_deref()).await?;

    let context_name = match &config.local_context {
        Some(name) => name.clone(),
        None => kubeconfig
            .current_context
            .clone()
            .ok_or_else(|| anyhow!("Kubeconfig has no current-context and PLATZ_LOCAL_CONTEXT is not set"))?,
    };

    let context = kubeconfig
        .contexts
        .iter()
        .find(|c| c.name == context_name)
        .ok_or_else(|| anyhow!("Context {context_name:?} not found in kubeconfig"))?;
    let context_inner = context
        .context
        .as_ref()
        .ok_or_else(|| anyhow!("Context {context_name:?} has no body"))?;

    let cluster_name = context_inner.cluster.clone();
    let user_name = context_inner
        .user
        .clone()
        .ok_or_else(|| anyhow!("Context {context_name:?} has no user"))?;

    let cluster = kubeconfig
        .clusters
        .iter()
        .find(|c| c.name == cluster_name)
        .ok_or_else(|| anyhow!("Cluster {cluster_name:?} not found in kubeconfig"))?
        .clone();
    let auth_info = kubeconfig
        .auth_infos
        .iter()
        .find(|a| a.name == user_name)
        .ok_or_else(|| anyhow!("User {user_name:?} not found in kubeconfig"))?
        .clone();

    let scoped_kubeconfig = kube::config::Kubeconfig {
        api_version: kubeconfig.api_version.clone(),
        kind: kubeconfig.kind.clone(),
        preferences: kubeconfig.preferences.clone(),
        current_context: Some(context_name.clone()),
        contexts: vec![context.clone()],
        clusters: vec![cluster],
        auth_infos: vec![auth_info],
        extensions: kubeconfig.extensions.clone(),
    };

    info!("Registering local cluster from context {context_name:?}");

    Ok(K8s::Local(Box::new(LocalCluster {
        name: context_name.clone(),
        provider_id: format!("local:{context_name}"),
        kubeconfig: scoped_kubeconfig,
    })))
}

async fn load_local_kubeconfig(explicit_path: Option<&std::path::Path>) -> Result<kube::config::Kubeconfig> {
    if let Some(path) = explicit_path {
        debug!("Loading kubeconfig from {}", path.display());
        return Ok(kube::config::Kubeconfig::read_from(path)?);
    }
    debug!("Loading kubeconfig from KUBECONFIG / default location");
    Ok(kube::config::Kubeconfig::read()?)
}
