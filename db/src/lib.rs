#![recursion_limit = "256"]

mod ui_collection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
pub use ui_collection::DbTableOrDeploymentResource;

mod config;
use crate::config::database_url;

mod db_table;
pub use db_table::*;

mod errors;
pub use errors::*;

mod schema;
pub use schema::*;

mod events;
pub use events::*;

pub mod json_diff;

pub use async_diesel::*;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
pub use diesel_json::Json;
use lazy_static::lazy_static;
use log::*;
use tokio::task;

mod pagination;
pub use pagination::{Paginated, DEFAULT_PAGE_SIZE};

mod identity;
pub use identity::Identity;

type PoolManager = ConnectionManager<PgConnection>;

pub type DbPool = Pool<PoolManager>;

pub struct Db {
    pool: DbPool,
    events: DbEventBroadcast,
}

impl Db {
    fn new() -> Self {
        let url = database_url();
        info!("Connecting to {}", url);
        let pool = Pool::builder().build(PoolManager::new(url)).unwrap();
        let events = Default::default();
        Self { pool, events }
    }
}

lazy_static! {
    static ref DB: Db = Db::new();
}

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub async fn init_db(run_migrations: bool) -> DbResult<()> {
    if run_migrations {
        info!("Running database migrations");
        let mut conn = DB.pool.get()?;
        conn.run_pending_migrations(MIGRATIONS).unwrap();
        info!("Finished running migrations");
    }
    task::spawn(DB.events.run());
    Ok(())
}

pub fn pool() -> &'static DbPool {
    &DB.pool
}

pub fn db_events() -> DbEventReceiver {
    DB.events.subscribe()
}
