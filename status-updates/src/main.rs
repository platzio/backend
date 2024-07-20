mod events;
mod status_config;
mod tracker;

use crate::tracker::StatusTracker;
use anyhow::Result;
use platz_db::DbTable;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("Starting status updates worker");

    platz_db::init_db(
        false,
        platz_db::NotificationListeningOpts::on_table(DbTable::Deployments),
    )
    .await?;
    events::watch_deployments(StatusTracker::new()).await
}
