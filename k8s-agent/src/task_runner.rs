use crate::{deploy::RunnableDeploymentTask, k8s::K8S_TRACKER};
use anyhow::Result;
use log::*;
use platz_db::{db_events, DbEvent, DbEventOperation, DbTable, DeploymentTask};
use tokio::{select, sync::watch};

pub async fn start() -> Result<()> {
    let (db_changes_tx, mut db_changes_rx) = watch::channel(());
    let mut k8s_tracker_rx = K8S_TRACKER.rx().await;

    db_changes_tx.send(())?;

    tokio::spawn(async move {
        let mut db_rx = db_events();
        while let Ok(event) = db_rx.recv().await {
            debug!("Got {:?}", event);
            if is_new_task(&event) {
                db_changes_tx.send(()).unwrap();
            }
        }
    });

    loop {
        info!("Waiting for next event...");
        select! {
            db_event = db_changes_rx.changed() => {
                info!("Got DB event: {:?}", db_event);
                db_event?;
                run_pending_tasks().await?;
            }
            k8s_event = k8s_tracker_rx.recv() => {
                info!("Got K8S_TRACKER event: {:?}", k8s_event);
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
