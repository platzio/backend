#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

mod ui_collection;
pub use ui_collection::DbTableOrDeploymentResource;

mod config;
use crate::config::database_url;

mod db_table;
pub use db_table::*;

mod errors;
pub use errors::*;

mod models;
pub use models::*;

mod events;
pub use events::*;

mod json_diff;

pub use async_diesel::*;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use lazy_static::lazy_static;
use log::*;
use tokio::task;

pub use diesel_json;

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

embed_migrations!();

pub async fn init_db(run_migrations: bool) -> DbResult<()> {
    if run_migrations {
        info!("Running database migrations");
        embedded_migrations::run(&DB.pool.get().unwrap()).unwrap();
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
