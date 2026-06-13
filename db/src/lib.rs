mod config;
mod db_table;
mod errors;
mod events;
mod identity;
pub mod json_diff;
pub mod schema;
mod stats;
mod tls;
mod ui_collection;

use crate::config::{DbPoolOptions, SslSettings, database_url, db_pool_options};
pub use db_table::*;
use diesel_async::{
    AsyncPgConnection,
    async_connection_wrapper::AsyncConnectionWrapper,
    pooled_connection::{
        AsyncDieselConnectionManager, ManagerConfig,
        bb8::{Pool, PooledConnection},
    },
};
pub use diesel_filter;
pub use diesel_json::Json;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
pub use diesel_pagination;
pub use errors::*;
pub use events::*;
pub use identity::Identity;
use tokio::{
    spawn,
    sync::OnceCell,
    task::{JoinHandle, spawn_blocking},
};
use tracing::info;
pub use ui_collection::DbTableOrDeploymentResource;

pub type DbPool = Pool<AsyncPgConnection>;
pub type DbConn = PooledConnection<'static, AsyncPgConnection>;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub struct Db {
    pool: DbPool,
    events: DbEventBroadcast,
    _stats_task: JoinHandle<()>,
}

impl Db {
    async fn new(pool_options: DbPoolOptions) -> DbResult<Self> {
        let connection_url = database_url();
        let ssl = SslSettings::from_env().map_err(errors::DbError::SslConfigError)?;
        info!("Connecting to {connection_url} (sslmode={:?})", ssl.mode);

        // Wire the TLS connector into every pooled connection via a custom
        // setup callback, so the pool negotiates TLS exactly like the
        // LISTEN/NOTIFY connection in `events.rs`.
        let connector = tls::build_connector(&ssl)?;
        let mode = ssl.mode;
        let mut manager_config = ManagerConfig::default();
        manager_config.custom_setup = Box::new(move |url| {
            let connector = connector.clone();
            let url = url.to_string();
            Box::pin(async move { tls::establish_connection(&url, connector, mode).await })
        });
        let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new_with_config(
            connection_url,
            manager_config,
        );
        let pool = Pool::builder()
            .max_size(pool_options.max_size)
            .min_idle(pool_options.min_idle)
            .connection_timeout(pool_options.connection_timeout)
            .idle_timeout(pool_options.idle_timeout)
            .max_lifetime(pool_options.max_lifetime)
            .build(config)
            .await?;
        info!("Pool configuration: {:?}", pool_options);
        let events = Default::default();
        let stats_task = spawn(stats::start(pool.clone()));
        Ok(Self {
            pool,
            events,
            _stats_task: stats_task,
        })
    }

    pub async fn run_migrations(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Running database migrations");
        let conn = self.pool.dedicated_connection().await?;
        let mut async_wrapper = AsyncConnectionWrapper::<AsyncPgConnection>::from(conn);
        spawn_blocking(move || async_wrapper.run_pending_migrations(MIGRATIONS).map(|_| ()))
            .await??;
        info!("Finished running migrations");
        Ok(())
    }

    pub async fn serve_db_events(
        &self,
        opts: NotificationListeningOpts,
    ) -> Result<(), DbEventsError> {
        self.events.run(opts).await
    }

    pub fn subscribe_to_events(&self) -> DbEventReceiver {
        self.events.subscribe()
    }
}

static DB: OnceCell<Db> = OnceCell::const_new();

pub async fn init_db() -> DbResult<&'static Db> {
    DB.get_or_try_init(|| Db::new(db_pool_options())).await
}

pub fn db() -> DbResult<&'static Db> {
    DB.get().ok_or(DbError::DbNotInitialized)
}

pub(crate) async fn db_conn() -> DbResult<DbConn> {
    Ok(db()?.pool.get().await?)
}
