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

    select! {
        _ = sigterm.recv() => {
            warn!("SIGTERM received, exiting");
            Ok(())
        }

        _ = sigint.recv() => {
            warn!("SIGINT received, exiting");
            Ok(())
        }

        result = platz_db::serve_db_events(
            platz_db::NotificationListeningOpts::on_table(
                DbTable::Deployments,
            ),
        ) => {
            warn!("DB events task exited: {result:?}");
            result.map_err(Into::into)
        }

        result = events::watch_deployments(StatusTracker::new()) => {
            result
        }
    }
}
