use crate::tracker::StatusTracker;
use anyhow::Result;
use futures::future::join_all;
use log::*;
use platz_db::{db_events, DbEventOperation, DbTable, Deployment};

pub async fn watch_deployments(tracker: StatusTracker) -> Result<()> {
    let mut db_rx = db_events();

    join_all(
        Deployment::all()
            .await?
            .into_iter()
            .map(|deployment| tracker.add(deployment)),
    )
    .await;

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
