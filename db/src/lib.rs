mod config;
mod db_table;
mod errors;
mod events;
mod identity;
pub mod json_diff;
mod pagination;
mod schema;
mod stats;
mod ui_collection;

use crate::config::database_url;
pub use db_table::*;
use diesel_async::{
    async_connection_wrapper::AsyncConnectionWrapper,
    pooled_connection::{
        bb8::{Pool, PooledConnection},
        AsyncDieselConnectionManager,
    },
    AsyncPgConnection,
};
pub use diesel_json::Json;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
pub use errors::*;
pub use events::*;
pub use identity::Identity;
pub use pagination::{Paginated, DEFAULT_PAGE_SIZE};
pub use schema::*;
use tokio::{
    spawn,
    sync::OnceCell,
    task::{spawn_blocking, JoinHandle},
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
    async fn new() -> Self {
        let connection_url = database_url();
        info!("Connecting to {connection_url}");
        let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(connection_url);
        let pool = Pool::builder().build(config).await.unwrap();
        let events = Default::default();
        let stats_task = spawn(stats::start(pool.clone()));
        Self {
            pool,
            events,
            _stats_task: stats_task,
        }
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

pub async fn init_db() -> &'static Db {
    DB.get_or_init(Db::new).await
}

pub fn db() -> &'static Db {
    DB.get().unwrap()
}

pub(crate) async fn db_conn() -> DbResult<DbConn> {
    Ok(db().pool.get().await?)
}
