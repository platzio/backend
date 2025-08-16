use crate::status_config::StatusConfig;
use anyhow::Result;
use futures::TryFutureExt;
use platz_db::schema::{
    deployment::{Deployment, UpdateDeploymentReportedStatus},
    deployment_status::{DeploymentReportedStatus, DeploymentReportedStatusContent},
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{sync::RwLock, task};
use tracing::{error, info, warn};
use url::Url;
use uuid::Uuid;

#[derive(Default, Clone)]
pub struct StatusTracker {
    inner: Arc<Inner>,
}

#[derive(Default)]
struct Inner {
    configs: RwLock<HashMap<Uuid, StatusConfig>>,
    tasks: RwLock<HashMap<Uuid, task::JoinHandle<()>>>,
}

impl StatusTracker {
    pub fn new() -> Self {
        Default::default()
    }

    pub async fn add(&self, deployment: Deployment) {
        if !deployment.enabled {
            warn!(
                "Deployment {} is disabled, stopping its status updates if there were any",
                deployment.id
            );
            self.remove(deployment.id).await;
            if deployment.reported_status.is_some() {
                UpdateDeploymentReportedStatus::new(None)
                    .save(deployment.id)
                    .await
                    .unwrap();
            }
            return;
        }

        let new_config = match StatusConfig::new(&deployment).await {
            Ok(new_config) => new_config,
            Err(err) => {
                error!(
                    "Could not get deployment status config for {}: {}",
                    deployment.id, err
                );
                self.remove(deployment.id).await;
                return;
            }
        };

        let mut configs = self.inner.configs.write().await;
        if let Some(cur_config) = configs.get(&deployment.id)
            && new_config == *cur_config
        {
            info!(
                "Deployment {} status config hasn't changed, doing nothing",
                deployment.id
            );
            return;
        }

        info!(
            "Starting to update status for deployment {} ({})",
            deployment.id,
            deployment.namespace_name().await
        );

        configs.insert(deployment.id, new_config.clone());

        let mut tasks = self.inner.tasks.write().await;
        if let Some(handle) = tasks.insert(
            deployment.id,
            task::spawn(periodic_deployment_status_update(deployment, new_config)),
        ) {
            handle.abort();
        }
    }

    pub async fn remove(&self, id: Uuid) {
        info!("Removing deployment {}", id);
        self.inner.configs.write().await.remove(&id);
        if let Some(handle) = self.inner.tasks.write().await.remove(&id) {
            handle.abort();
        }
    }
}

async fn get_deployment_reported_status(url: &Url) -> Result<DeploymentReportedStatusContent> {
    Ok(reqwest::Client::new()
        .get(url.to_owned())
        .timeout(Duration::from_secs(10))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

async fn periodic_deployment_status_update(deployment: Deployment, status_config: StatusConfig) {
    let mut interval = status_config.interval();

    loop {
        interval.tick().await;

        let reported_status = get_deployment_reported_status(&status_config.url)
            .map_ok_or_else(
                DeploymentReportedStatus::new_error,
                DeploymentReportedStatus::new,
            )
            .await;

        let update_result = UpdateDeploymentReportedStatus::new(Some(reported_status))
            .save(deployment.id)
            .await;

        if let Err(err) = update_result {
            error!("Error updating deployment: {}", err);
        }
    }
}
