use crate::tracker::StatusTracker;
use anyhow::Result;
use futures::future::join_all;
use platz_db::{db_events, DbEventOperation, DbTable, Deployment};
use tokio::time;
use tracing::debug;

const DEPLOYMENT_CHUNK_SIZE: usize = 10;
const DEPLOYMENT_SLEEP_BETWEEN_CHUNKS: time::Duration = time::Duration::from_secs(1);

pub async fn watch_deployments(tracker: StatusTracker) -> Result<()> {
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
        if event.table == DbTable::Deployments {
            match event.operation {
                DbEventOperation::Delete => tracker.remove(event.data.id).await,
                _ => {
                    let deployment = Deployment::find(event.data.id).await?;
                    tracker.add(deployment).await
                }
            }
        }
    }
}
