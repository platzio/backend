mod helm;
mod install_and_upgrade;
mod invoke_action;
mod restart_k8s_resource;
mod runnable_task;
mod secrets;
mod values;

use crate::k8s::K8S_TRACKER;
use crate::utils::create_interval_stream;
use anyhow::Result;
use futures::StreamExt;
use platz_db::{db_events, DbEvent, DbEventOperation, DbTable, DeploymentTask};
use runnable_task::RunnableDeploymentTask;
pub use secrets::apply_secret;
use tokio::{select, sync::watch};
use tracing::{debug, error, info, Instrument};

#[tracing::instrument(err, name = "task_runner")]
pub async fn start() -> Result<()> {
    let (db_events_tx, mut db_events_rx) = watch::channel(());
    let mut k8s_events_rx = K8S_TRACKER.outbound_notifications_rx().await;

    tokio::spawn(
        async move {
            let mut db_rx = db_events();
            while let Ok(event) = db_rx.recv().await {
                tracing::debug!(?event);
                if is_new_task(&event) {
                    tracing::debug!("Task detected");
                    db_events_tx.send(()).unwrap();
                }
            }
        }
        .instrument(tracing::debug_span!("db-events")),
    );

    debug!("starting poll loop");
    let mut updates_interval_stream = create_interval_stream(std::time::Duration::from_secs(60));

    loop {
        run_pending_tasks().await?;
        debug!("polling...");
        select! {
            biased;

            db_event = db_events_rx.changed() => {
                debug!("db task event received");
                db_event?;
            }
            k8s_event = k8s_events_rx.changed() => {
                tracing::debug!("k8s event received");
                k8s_event?;
            }

            _ = updates_interval_stream.next() => {
                // Stop waiting for events, pull pending tasks
            }
        }
    }
}

fn is_new_task(event: &DbEvent) -> bool {
    event.table == DbTable::DeploymentTasks && event.operation == DbEventOperation::Insert
}

#[tracing::instrument(err)]
async fn run_pending_tasks() -> Result<()> {
    use std::time::Instant;

    debug!("fetching clusters...");
    let cluster_ids = K8S_TRACKER.get_ids().await;
    debug!("fetching tasks...");
    let mut fetch_start_time = Instant::now();
    while let Some(task) = DeploymentTask::next_pending(&cluster_ids).await? {
        let task_id = task.id;

        debug!(
            "Fetched task {task_id}, fetch took {:?}",
            Instant::now().duration_since(fetch_start_time)
        );

        let span = tracing::debug_span!("task", task_id = %task_id);

        async move {
            info!("Starting...");
            match task.run().await {
                Ok(()) => debug!("Task finished successfully"),
                Err(err) => error!("Task failed: {:?}", err),
            }
        }
        .instrument(span)
        .await;
        fetch_start_time = Instant::now();
        debug!("Fetching tasks...");
    }
    debug!("No pending tasks");
    Ok(())
}
