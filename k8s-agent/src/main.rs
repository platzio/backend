mod config;
mod deployment_creds;
mod k8s;
mod task_runner;
mod utils;

use crate::config::CONFIG;
use anyhow::Result;
use platz_db::DbTable;
use tokio::{
    select,
    signal::unix::{signal, SignalKind},
};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("Starting K8S Agent");
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;

    platz_db::init_db(
        false,
        platz_db::NotificationListeningOpts::on_table(DbTable::DeploymentTasks),
    )
    .await?;

    select! {
        _ = sigterm.recv() => {
            warn!("SIGTERM received, exiting");
            Ok(())
        }

        _ = sigint.recv() => {
            warn!("SIGINT received, exiting");
            Ok(())
        }

        result = k8s::scan_for_new_clusters(CONFIG.k8s_refresh_interval()) => {
            warn!("EKS discovery task finished");
            result
        }

        result = task_runner::start() => {
            warn!("Task runner finished");
            result
        }

        result = deployment_creds::start(CONFIG.should_refresh_deployment_credintials()) => {
            warn!("Deployment creds task finished");
            result
        }
    }
}
