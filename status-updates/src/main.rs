mod events;
mod status_config;
mod tracker;

use crate::tracker::StatusTracker;
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
    info!("Starting status updates worker");
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;

    platz_db::init_db(
        false,
        platz_db::NotificationListeningOpts::on_table(DbTable::Deployments),
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

        result = events::watch_deployments(StatusTracker::new()) => {
            result
        }
    }
}
