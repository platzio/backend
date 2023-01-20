use anyhow::{Context, Result};
use lazy_static::lazy_static;
use log::*;
use platz_db::{Deployment, DeploymentTask, K8sCluster};
use prometheus::{register_int_gauge_vec, IntGaugeVec};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

lazy_static! {
    pub static ref TASK_STATUS_COUNTERS: IntGaugeVec = register_int_gauge_vec!(
        "platz_deployment_task_status_counter",
        "Number of deployment tasks of each status",
        &["task_status"],
    )
    .unwrap();
    pub static ref DEPLOYMENT_STATUS_COUNTERS: IntGaugeVec = register_int_gauge_vec!(
        "platz_deployment_status_counter",
        "Number of deployment tasks of each status per status",
        &["deployment_kind", "deployment_status", "cluster_name"],
    )
    .unwrap();
}

pub(crate) fn initialize() {}

async fn update_metrics() -> Result<()> {
    let task_status_counts = DeploymentTask::get_status_counters()
        .await
        .context("Failed updating prometheus metrics of deployment tasks")?;
    for stat in task_status_counts.iter() {
        TASK_STATUS_COUNTERS
            .with_label_values(&[stat.status.as_str()])
            .set(stat.count);
    }

    let cluster_id_to_cluster_name: HashMap<Uuid, String> = K8sCluster::all()
        .await
        .context("Failed fetching k8s clusters")?
        .iter()
        .map(|cluster| (cluster.id, cluster.name.clone()))
        .collect();

    let deployment_status_counts = Deployment::get_status_counters()
        .await
        .context("Failed updating prometheus metrics deployments")?;

    for stat in deployment_status_counts.iter() {
        let cluster_name = cluster_id_to_cluster_name
            .get(&stat.cluster_id)
            .cloned()
            .unwrap_or_else(|| "N/A".to_string());

        DEPLOYMENT_STATUS_COUNTERS
            .with_label_values(&[
                stat.kind.as_str(),
                stat.status.as_str(),
                cluster_name.as_str(),
            ])
            .set(stat.count);
    }

    Ok(())
}

pub async fn update_metrics_task(update_interval: Duration) -> Result<()> {
    let mut interval = time::interval(update_interval);

    loop {
        interval.tick().await;
        update_metrics().await.map_err(|err| {
            error!("ERROR: {err:?}");
            err
        })?;
    }
}
