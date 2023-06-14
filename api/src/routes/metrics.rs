use anyhow::{Context, Result};
use lazy_static::lazy_static;
use platz_db::{Deployment, DeploymentStatus, DeploymentTask, DeploymentTaskStatus, K8sCluster};
use prometheus::{register_int_gauge_vec, IntGaugeVec};
use std::collections::HashMap;
use std::time::Duration;
use strum::IntoEnumIterator;
use tokio::time;

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

async fn update_metrics() -> Result<()> {
    let task_status_counts = DeploymentTask::get_status_counters()
        .await
        .context("Failed updating prometheus metrics of deployment tasks")?;

    TASK_STATUS_COUNTERS.reset();
    for status in DeploymentTaskStatus::iter() {
        TASK_STATUS_COUNTERS
            .with_label_values(&[status.as_ref()])
            .set(0);
    }

    for stat in task_status_counts.iter() {
        TASK_STATUS_COUNTERS
            .with_label_values(&[stat.status.as_ref()])
            .set(stat.count);
    }

    let k8s_clusters = K8sCluster::all()
        .await
        .context("Failed fetching k8s clusters")?;
    let cluster_id_to_cluster_name: HashMap<_, _> = k8s_clusters
        .iter()
        .map(|cluster| (cluster.id, cluster.name.as_str()))
        .collect();
    let deployment_status_counts = Deployment::get_status_counters()
        .await
        .context("Failed updating prometheus metrics deployments")?;

    DEPLOYMENT_STATUS_COUNTERS.reset();
    for status in DeploymentStatus::iter() {
        DEPLOYMENT_STATUS_COUNTERS
            .with_label_values(&[status.as_ref()])
            .set(0);
    }
    for stat in deployment_status_counts.into_iter() {
        let cluster_name = cluster_id_to_cluster_name
            .get(&stat.cluster_id)
            .cloned()
            .unwrap_or("");
        DEPLOYMENT_STATUS_COUNTERS
            .with_label_values(&[stat.kind.as_str(), stat.status.as_ref(), cluster_name])
            .set(stat.count);
    }

    Ok(())
}

pub async fn update_metrics_task(update_interval: Duration) -> Result<()> {
    let mut interval = time::interval(update_interval);

    loop {
        interval.tick().await;
        update_metrics().await?
    }
}
