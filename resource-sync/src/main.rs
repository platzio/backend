mod task;

use crate::task::{monitor_deployment_resource_changes, scrub_deployment_resources};
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
    info!("Starting deployment resource sync worker");
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;

    let db = init_db().await;

    let fut = tokio::spawn(monitor_deployment_resource_changes(db));

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

        result = db.serve_db_events(
            platz_db::NotificationListeningOpts::on_table(
                DbTable::DeploymentResources,
            ),
        ) => {
            warn!("DB events task exited: {result:?}");
            result.map_err(Into::into)
        }

        result = fut => {
            result?
        }
    }
}
