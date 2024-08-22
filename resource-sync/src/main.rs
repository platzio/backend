mod task;

use crate::task::{monitor_deployment_resource_changes, scrub_deployment_resources};
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
    info!("Starting deployment resource sync worker");
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;

    platz_db::init_db(
        false,
        platz_db::NotificationListeningOpts::on_table(DbTable::DeploymentResources),
    )
    .await?;

    let fut = tokio::spawn(monitor_deployment_resource_changes());

    info!("Scrubbing all existing deployment resources");
    scrub_deployment_resources().await?;
    info!("Finished scrubbing, will now watch for changes");

    select! {
        _ = sigterm.recv() => {
            warn!("SIGTERM received, exiting");
            Ok(())
        }

        _ = sigint.recv() => {
            warn!("SIGINT received, exiting");
            Ok(())
        }

        result = fut => {
            result?
        }
    }
}
