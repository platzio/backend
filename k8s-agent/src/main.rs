mod config;
mod deployment_creds;
mod k8s;
mod task_runner;
mod utils;

use crate::config::CONFIG;
use anyhow::Result;
use log::*;
use platz_db::DbTable;

pub async fn _main() -> Result<()> {
    platz_db::init_db(
        false,
        platz_db::NotificationListeningOpts::on_table(DbTable::DeploymentTasks),
    )
    .await?;

    tokio::select! {
        result = k8s::scan_for_new_clusters(CONFIG.k8s_refresh_interval()) => {
            warn!("EKS discovery task finished");
            result?;
        }

        result = task_runner::start() => {
            warn!("Task runner finished");
            result?;
        }

        result = deployment_creds::start(CONFIG.should_refresh_deployment_credintials()) => {
            warn!("Deployment creds task finished");
            result?;
        }
    };

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    info!("Starting K8S Agent");

    _main().await
}
