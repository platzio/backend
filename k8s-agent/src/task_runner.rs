use crate::{deploy::RunnableDeploymentTask, k8s::K8S_TRACKER};
use anyhow::{anyhow, Result};
use log::*;
use platz_db::{db_events, DbEvent, DbEventOperation, DbTable, DeploymentTask};
use tokio::sync::watch;

pub async fn start() -> Result<()> {
    let (tx, mut rx) = watch::channel(());

    tx.send(())?;

    tokio::spawn(async move {
        let mut db_rx = db_events();
        while let Ok(event) = db_rx.recv().await {
            debug!("Got {:?}", event);
            if is_new_task(&event) {
                tx.send(()).unwrap();
            }
        }
    });

    while rx.changed().await.is_ok() {
        run_pending_tasks().await?;
        info!("Waiting for tasks to run...");
    }

    Err(anyhow!(
        "Deployer task finished, this isn't supposed to happen"
    ))
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
