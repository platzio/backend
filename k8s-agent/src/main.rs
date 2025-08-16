mod config;
mod deployment_creds;
mod k8s;
mod task_runner;
mod utils;

use crate::{config::Config, k8s::cluster_discovery::run_cluster_discovery};
use anyhow::Result;
use clap::Parser;
use platz_db::{DbTable, init_db};
use tokio::{
    select,
    signal::unix::{SignalKind, signal},
};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::parse();

    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed installing default crypto provider");
    platz_otel::init()?;
    info!("Starting K8S Agent");
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;

    let db = init_db().await;

    select! {
        _ = sigterm.recv() => {
            warn!("SIGTERM received, exiting");
            Ok(())
        }

        _ = sigint.recv() => {
            warn!("SIGINT received, exiting");
            Ok(())
        }

        result = db.serve_db_events(
            platz_db::NotificationListeningOpts::on_table(
                DbTable::DeploymentTasks,
            ),
        ) => {
            warn!("DB events task exited: {result:?}");
            result.map_err(Into::into)
        }

        result = run_cluster_discovery(&config.cluster_discovery) => {
            warn!("EKS discovery task finished");
            result
        }

        result = task_runner::start(&config, db) => {
            warn!("Task runner finished");
            result
        }

        result = deployment_creds::start(&config) => {
            warn!("Deployment creds task finished");
            result
        }
    }
}
