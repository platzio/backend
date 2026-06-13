use crate::DbTable;
use crate::config::SslSettings;
use serde::{Deserialize, Serialize};
use std::{future::poll_fn, task::ready};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    spawn,
    sync::broadcast,
    task::JoinHandle,
    time,
};
use tokio_postgres::{AsyncMessage, Connection, NoTls};
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
    /// Environment the changed row belongs to, resolved by the database trigger.
    /// `None` for rows that are not environment-scoped (global tables) or whose
    /// environment could not be resolved. Used to forward each event only to
    /// clients permitted to see that environment.
    #[serde(default)]
    #[schema(required)]
    pub env_id: Option<Uuid>,
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
    #[error("Invalid database TLS configuration: {0}")]
    SslConfigError(String),
    #[error("Database TLS error: {0}")]
    TlsError(crate::tls::TlsError),
}

impl DbEventsError {
    fn retryable(&self) -> bool {
        match self {
            Self::TokioJoinError(_) => false,
            Self::EventParseError(_) => false,
            Self::ConnectError(_) => true,
            Self::PollError(_) => true,
            Self::ListenQueryFailed(_) => false,
            // Misconfiguration won't fix itself on retry.
            Self::SslConfigError(_) => false,
            Self::TlsError(_) => false,
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
        let url = crate::config::database_url();
        let ssl = SslSettings::from_env().map_err(DbEventsError::SslConfigError)?;

        // Establish the dedicated LISTEN/NOTIFY connection using the same TLS
        // settings as the connection pool. With TLS disabled we keep the
        // original plaintext (`NoTls`) path.
        let (client, events_task) =
            match crate::tls::build_connector(&ssl).map_err(DbEventsError::TlsError)? {
                None => {
                    let (client, connection) = tokio_postgres::connect(&url, NoTls)
                        .await
                        .map_err(DbEventsError::ConnectError)?;
                    (client, spawn_event_pump(connection, events_tx))
                }
                Some(connector) => {
                    let mut config: tokio_postgres::Config =
                        url.parse().map_err(DbEventsError::ConnectError)?;
                    config.ssl_mode(crate::tls::pg_ssl_mode(ssl.mode));
                    let (client, connection) = config
                        .connect(connector)
                        .await
                        .map_err(DbEventsError::ConnectError)?;
                    (client, spawn_event_pump(connection, events_tx))
                }
            };

        client
            .execute(&format!("LISTEN {channel_name}"), &[])
            .await
            .map_err(DbEventsError::ListenQueryFailed)?;

        events_task.await?
    }
}

/// Pumps `LISTEN`/`NOTIFY` messages off a Postgres connection and rebroadcasts
/// them as [`DbEvent`]s. Generic over the connection's stream type so it works
/// with both the plaintext (`NoTls`) and TLS-wrapped connections.
fn spawn_event_pump<S, T>(
    mut connection: Connection<S, T>,
    events_tx: DbEventSender,
) -> JoinHandle<Result<(), DbEventsError>>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    spawn(poll_fn(move |cx| {
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
    }))
}
