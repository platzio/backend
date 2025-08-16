use crate::DbTable;
use serde::{Deserialize, Serialize};
use std::{future::poll_fn, task::ready};
use tokio::{spawn, sync::broadcast, time};
use tokio_postgres::AsyncMessage;
use tracing::{debug, error, trace};
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

pub struct NotificationListeningOpts {
    channel_name: String,
}

impl NotificationListeningOpts {
    pub fn on_table(table_name: DbTable) -> Self {
        Self {
            channel_name: format!("db_{table_name}_notifications"),
        }
    }
    pub fn all() -> Self {
        Self {
            channel_name: "db_notifications".to_string(),
        }
    }
}

pub struct DbEventBroadcast {
    tx: DbEventSender,
}

impl Default for DbEventBroadcast {
    fn default() -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self { tx }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DbEventsError {
    #[error("Tokio join error: {0}")]
    TokioJoinError(#[from] tokio::task::JoinError),
    #[error("Event parse error: {0}")]
    EventParseError(serde_json::Error),
    #[error("Database connection error: {0}")]
    ConnectError(tokio_postgres::Error),
    #[error("Poll connection error: {0}")]
    PollError(tokio_postgres::Error),
    #[error("Error running LISTEN query: {0}")]
    ListenQueryFailed(tokio_postgres::Error),
}

impl DbEventsError {
    fn retryable(&self) -> bool {
        match self {
            Self::TokioJoinError(_) => false,
            Self::EventParseError(_) => false,
            Self::ConnectError(_) => true,
            Self::PollError(_) => true,
            Self::ListenQueryFailed(_) => false,
        }
    }
}

impl DbEventBroadcast {
    pub fn subscribe(&self) -> DbEventReceiver {
        self.tx.subscribe()
    }

    pub async fn run(&self, opts: NotificationListeningOpts) -> Result<(), DbEventsError> {
        let channel_name = &opts.channel_name;
        loop {
            let listen_to = channel_name.clone();
            debug!("Listening for {}", &listen_to);
            match self.listen_for_notifications(&listen_to).await {
                Ok(()) => continue,
                Err(err) if err.retryable() => {
                    error!("Retryable error while listening for notifications: {err:?}");
                    time::sleep(time::Duration::from_secs(3)).await;
                }
                Err(err) => break Err(err),
            }
        }
    }

    async fn listen_for_notifications(&self, channel_name: &str) -> Result<(), DbEventsError> {
        let events_tx = self.tx.clone();
        let (client, mut connection) =
            tokio_postgres::connect(&crate::config::database_url(), tokio_postgres::NoTls)
                .await
                .map_err(DbEventsError::ConnectError)?;

        let events_task = spawn(poll_fn(move |cx| {
            loop {
                while let Some(message) = ready!(
                    connection
                        .poll_message(cx)
                        .map_err(DbEventsError::PollError)?
                ) {
                    match message {
                        AsyncMessage::Notice(notice) => {
                            trace!("Database notice: {notice:?}");
                        }
                        AsyncMessage::Notification(notification) => {
                            let event: DbEvent = serde_json::from_str(notification.payload())
                                .map_err(DbEventsError::EventParseError)?;
                            events_tx.send(event).ok();
                        }
                        other => {
                            trace!("Got unknown message from Postgres: {other:?}");
                        }
                    }
                }
            }
        }));

        client
            .execute(&format!("LISTEN {channel_name}"), &[])
            .await
            .map_err(DbEventsError::ListenQueryFailed)?;

        events_task.await?
    }
}
