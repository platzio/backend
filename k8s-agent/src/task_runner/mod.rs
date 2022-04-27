mod helm;
mod install_and_upgrade;
mod invoke_action;
mod restart_k8s_resource;
mod runnable_task;
mod secrets;
mod values;

use crate::k8s::K8S_TRACKER;
use anyhow::Result;
use log::*;
use platz_db::{db_events, DbEvent, DbEventOperation, DbTable, DeploymentTask};
use runnable_task::RunnableDeploymentTask;
pub use secrets::apply_secret;
use tokio::{select, sync::watch};

pub async fn start() -> Result<()> {
    let (db_events_tx, mut db_events_rx) = watch::channel(());
    let mut k8s_events_rx = K8S_TRACKER.outbound_notifications_rx().await;

    tokio::spawn(async move {
        let mut db_rx = db_events();
        while let Ok(event) = db_rx.recv().await {
            debug!("Got {:?}", event);
            if is_new_task(&event) {
                db_events_tx.send(()).unwrap();
            }
        }
    });

    loop {
        info!("Waiting for next event...");
        select! {
            db_event = db_events_rx.changed() => {
                info!("Got DB event: {:?}", db_event);
                db_event?;
                run_pending_tasks().await?;
            }
            k8s_event = k8s_events_rx.changed() => {
                info!("Got K8S_TRACKER event: {:?}", k8s_event);
                k8s_event?;
                run_pending_tasks().await?;
            }
        }
    }
}

fn is_new_task(event: &DbEvent) -> bool {
    event.table == DbTable::DeploymentTasks && event.operation == DbEventOperation::Insert
}

async fn run_pending_tasks() -> Result<()> {
    let cluster_ids = K8S_TRACKER.get_ids().await;
    info!("Running pending tasks for {:?}", cluster_ids);
    while let Some(task) = DeploymentTask::next_pending(&cluster_ids).await? {
        info!("Running task {}", task.id);
        task.run().await?;
    }
    Ok(())
}
