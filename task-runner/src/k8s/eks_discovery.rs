use super::cluster_type::K8s;
use super::tracker::K8S_TRACKER;
use anyhow::{anyhow, Result};
use futures::future::try_join_all;
use log::*;
use rusoto_eks::{DescribeClusterRequest, Eks, EksClient};
use rusoto_utils::creds::rusoto_client;
use rusoto_utils::regions::{get_regions, Region};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;

pub async fn load_clusters() -> Result<()> {
    let tracker_tx = K8S_TRACKER.tx().await;

    for cluster in discover_clusters().await?.into_iter() {
        info!("Found {}", cluster);
        tracker_tx.send(Arc::new(cluster))?;
    }

    Ok(())
}

pub async fn scan_for_new_clusters(every: Duration) -> Result<()> {
    let mut interval = time::interval(every);

    loop {
        interval.tick().await;
        load_clusters().await?;
    }
}

async fn discover_clusters() -> Result<Vec<K8s>> {
    let regions = get_regions().await?;
    info!("Found regions: {:?}", regions);
    let results = try_join_all(regions.into_iter().map(get_clusters)).await?;
    Ok(results.into_iter().flatten().collect())
}

async fn get_clusters(region: Region) -> Result<Vec<K8s>> {
    debug!("Getting clusters of {:?}", region);
    let client = rusoto_client(env!("CARGO_PKG_NAME").into())?;
    let eks = EksClient::new_with_client(client, region.clone());

    let cluster_names = eks
        .list_clusters(Default::default())
        .await?
        .clusters
        .ok_or_else(|| anyhow!("Got empty clusters from list_clusters"))?;

    let eks_clusters = try_join_all(cluster_names.into_iter().map(|name| {
        debug!("Getting cluster info: {:?} {:?}", region, name);
        eks.describe_cluster(DescribeClusterRequest { name })
    }))
    .await?
    .into_iter()
    .map(|res| {
        res.cluster
            .ok_or_else(|| anyhow!("describe_cluster returned empty cluster"))
    })
    .collect::<std::result::Result<Vec<rusoto_eks::Cluster>, anyhow::Error>>()?;

    Ok(eks_clusters.into_iter().map(K8s::from).collect())
}
