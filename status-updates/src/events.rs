use crate::tracker::StatusTracker;
use anyhow::Result;
use futures::future::join_all;
use platz_db::{db_events, DbEventOperation, DbTable, Deployment};
use std::{io::Write, path::PathBuf};
use tokio::time;
use tracing::debug;

const DEPLOYMENT_CHUNK_SIZE: usize = 10;
const DEPLOYMENT_SLEEP_BETWEEN_CHUNKS: time::Duration = time::Duration::from_secs(1);

pub async fn watch_deployments(
    tracker: StatusTracker,
    heartbeat_file_path: Option<PathBuf>,
) -> Result<()> {
    let mut db_rx = db_events();

    for deploy_chunk in Deployment::all().await?.chunks(DEPLOYMENT_CHUNK_SIZE) {
        join_all(
            deploy_chunk
                .iter()
                .filter(|dep| dep.enabled)
                .map(|deployment| tracker.add(deployment.clone())),
        )
        .await;
        time::sleep(DEPLOYMENT_SLEEP_BETWEEN_CHUNKS).await;
    }

    loop {
        let event = db_rx.recv().await?;
        debug!("Got {:?}", event);
        tokio::time::timeout(
            std::time::Duration::from_secs(60),
            handle_db_event(&tracker, event),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Timed out processing DB event"))
        .inspect_err(|e| tracing::error!("Error processing DB events: {e:?}"))
        .and_then(|res| res)?;

        if let Some(path) = &heartbeat_file_path {
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(path)?;
            writeln!(&mut f, "{:?} -- {:?}", std::time::SystemTime::now(), event)?;
        }
    }
}

async fn handle_db_event(tracker: &StatusTracker, event: platz_db::DbEvent) -> Result<()> {
    if event.table == DbTable::Deployments {
        match event.operation {
            DbEventOperation::Delete => {
                tracker.remove(event.data.id).await;
            }
            _ => {
                let deployment = Deployment::find(event.data.id).await?;
                tracker.add(deployment).await;
            }
        }
    }
    Ok(())
}
