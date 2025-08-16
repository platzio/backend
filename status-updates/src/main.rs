mod events;
mod status_config;
mod tracker;

use crate::tracker::StatusTracker;
use anyhow::Result;
use platz_db::{DbTable, init_db};
use tokio::{
    select,
    signal::unix::{SignalKind, signal},
};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    platz_otel::init()?;
    info!("Starting status updates worker");
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
                DbTable::Deployments,
            ),
        ) => {
            warn!("DB events task exited: {result:?}");
            result.map_err(Into::into)
        }

        result = events::watch_deployments(db, StatusTracker::new()) => {
            result
        }
    }
}
