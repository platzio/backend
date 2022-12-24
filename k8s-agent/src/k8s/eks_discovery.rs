use super::cluster_type::K8s;
use super::tracker::K8S_TRACKER;
use anyhow::{anyhow, Result};
use aws_types::region::Region;
use futures::future::try_join_all;
use log::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;

pub async fn scan_for_new_clusters(every: Duration) -> Result<()> {
    let mut interval = time::interval(every);

    loop {
        interval.tick().await;
        load_clusters().await?;
    }
}

async fn load_clusters() -> Result<()> {
    let tracker_tx = K8S_TRACKER.inbound_requests_tx().await;

    for cluster in discover_clusters().await?.into_iter() {
        info!("Found {}", cluster);
        tracker_tx.send(Arc::new(cluster))?;
    }

    Ok(())
}

async fn discover_clusters() -> Result<Vec<K8s>> {
    let shared_config = aws_config::load_from_env().await;
    let ec2 = aws_sdk_ec2::Client::new(&shared_config);
    let regions = ec2
        .describe_regions()
        .send()
        .await?
        .regions
        .ok_or_else(|| anyhow!("Got an empty region list"))?
        .into_iter()
        .filter_map(|ec2_region| ec2_region.region_name().map(ToOwned::to_owned))
        .map(Region::new);
    info!("Found regions: {:?}", regions);
    let results = try_join_all(regions.map(get_clusters)).await?;
    Ok(results.into_iter().flatten().collect())
}

async fn get_clusters(region: Region) -> Result<Vec<K8s>> {
    debug!("Getting clusters of {:?}", region);
    let shared_config = aws_config::load_from_env().await;
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
        debug!("Getting cluster info: {:?} {:?}", region, name);
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
