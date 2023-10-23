mod task;

use crate::task::{monitor_deployment_resource_changes, scrub_deployment_resources};
use anyhow::Result;
use log::*;
use platz_db::DbTable;

pub async fn _main() -> Result<()> {
    platz_db::init_db(
        false,
        platz_db::NotificationListeningOpts::on_table(DbTable::DeploymentResources),
    )
    .await?;

    let fut = tokio::spawn(monitor_deployment_resource_changes());

    info!("Scrubbing all existing deployment resources");
    scrub_deployment_resources().await?;
    info!("Finished scrubbing, will now watch for changes");

    fut.await?
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    info!("Starting deployment resource sync worker");

    _main().await
}
