use anyhow::{Context, Result};
use lazy_static::lazy_static;
use platz_db::schema::{
    deployment::{Deployment, DeploymentStatus},
    deployment_kind::DeploymentKind,
    deployment_task::{DeploymentTask, DeploymentTaskStatus},
    k8s_cluster::K8sCluster,
};
use prometheus::{IntGaugeVec, register_int_gauge_vec};
use std::collections::HashMap;
use std::time::Duration;
use strum::IntoEnumIterator;
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

async fn update_metrics() -> Result<()> {
    let task_status_counts = DeploymentTask::get_status_counters()
        .await
        .context("Failed updating prometheus metrics of deployment tasks")?;
    let kind_id_to_name: HashMap<Uuid, String> = DeploymentKind::all()
        .await
        .context("Failed fetching deployment kinds")?
        .into_iter()
        .map(|deploy_kind| (deploy_kind.id, deploy_kind.name))
        .collect();

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
    for kind in deployment_status_counts
        .iter()
        .map(|stat| kind_id_to_name[&stat.kind_id].clone())
    {
        for status in DeploymentStatus::iter() {
            for cluster_name in cluster_id_to_cluster_name.values() {
                DEPLOYMENT_STATUS_COUNTERS
                    .with_label_values(&[kind.as_str(), status.as_ref(), cluster_name])
                    .set(0);
            }
        }
    }

    for stat in deployment_status_counts.into_iter() {
        let kind_name = kind_id_to_name[&stat.kind_id].clone();
        let cluster_name = cluster_id_to_cluster_name
            .get(&stat.cluster_id)
            .cloned()
            .unwrap_or("");
        DEPLOYMENT_STATUS_COUNTERS
            .with_label_values(&[kind_name.as_str(), stat.status.as_ref(), cluster_name])
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
