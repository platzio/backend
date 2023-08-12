use crate::DbError;
use crate::DbResult;
use crate::DbTable;
use log::*;
use postgres::fallible_iterator::FallibleIterator;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio::task;
use tokio::time;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum DbEventOperation {
    #[serde(rename = "INSERT")]
    Insert,
    #[serde(rename = "UPDATE")]
    Update,
    #[serde(rename = "DELETE")]
    Delete,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
pub struct DbEventData {
    pub id: Uuid,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
pub struct DbEvent {
    pub operation: DbEventOperation,
    pub table: DbTable,
    pub data: DbEventData,
}

pub type DbEventSender = broadcast::Sender<DbEvent>;
pub type DbEventReceiver = broadcast::Receiver<DbEvent>;

pub struct DbEventBroadcast {
    tx: DbEventSender,
}

impl Default for DbEventBroadcast {
    fn default() -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self { tx }
    }
}

impl DbEventBroadcast {
    pub fn subscribe(&self) -> DbEventReceiver {
        self.tx.subscribe()
    }

    pub async fn run(&self) {
        loop {
            debug!("Listening for db_notifications");
            let tx = self.tx.clone();
            match task::spawn_blocking(move || Self::listen_for_notifications(tx)).await {
                Ok(Ok(())) => continue,
                Ok(Err(err)) => {
                    error!("Error listening for notifications: {:?}", err);
                    time::sleep(time::Duration::from_secs(3)).await;
                }
                Err(err) => {
                    warn!("Stopping due to error while waiting for listen_for_notifications task: {:?}", err);
                    break;
                }
            }
        }
    }

    fn listen_for_notifications(tx: broadcast::Sender<DbEvent>) -> DbResult<()> {
        let mut client =
            postgres::Client::connect(&crate::config::database_url(), postgres::NoTls)?;
        client.execute("LISTEN db_notifications", &[])?;

        loop {
            let mut notifications = client.notifications();
            let mut iter = notifications.blocking_iter();

            while let Some(notification) = iter.next()? {
                let event: DbEvent = serde_json::from_str(notification.payload())
                    .map_err(DbError::EventParseError)?;
                tx.send(event).map_err(DbError::EventBroadcastError)?;
            }
        }
    }
}
