mod events;
mod status_config;
mod tracker;

use crate::tracker::StatusTracker;
use anyhow::Result;
use log::*;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("Starting status updates worker");

    platz_db::init_db(
        false,
        platz_db::NotificationListeningOpts::on_table("deployments"),
    )
    .await?;
    events::watch_deployments(StatusTracker::new()).await
}
