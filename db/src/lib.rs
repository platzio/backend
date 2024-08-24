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
use prometheus::{register_gauge, Gauge};
use tracing::info;

mod pagination;
pub use pagination::{Paginated, DEFAULT_PAGE_SIZE};

mod identity;
pub use identity::Identity;

lazy_static! {
    pub static ref DB_CHECKOUT_WAIT_DURATION_SUM: Gauge = register_gauge!(
        "platz_db_checkout_wait_duration_sum_seconds",
        "Number of total seconds spent waiting for db connection checkout"
    )
    .unwrap();
    pub static ref DB_CHECKOUT_DURATION_SUM: Gauge = register_gauge!(
        "platz_db_checkout_duration_sum_seconds",
        "Number of total seconds spent with checked out db connections"
    )
    .unwrap();
    pub static ref DB_NUM_CHECKOUTS: Gauge = register_gauge!(
        "platz_db_num_checked_out_connections",
        "Number of total checkouts that took place"
    )
    .unwrap();
    pub static ref DB_NUM_CHECKINS: Gauge = register_gauge!(
        "platz_db_num_checked_in_connections",
        "Number of total checkins that took place"
    )
    .unwrap();
}

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
        let pool = Pool::builder()
            .event_handler(Box::new(MetricEventHandler))
            .build(PoolManager::new(url))
            .unwrap();
        let events = Default::default();
        Self { pool, events }
    }
}

#[derive(Debug)]
struct MetricEventHandler;

impl r2d2::event::HandleEvent for MetricEventHandler {
    fn handle_checkout(&self, event: r2d2::event::CheckoutEvent) {
        DB_NUM_CHECKOUTS.inc();
        DB_CHECKOUT_WAIT_DURATION_SUM.add(event.duration().as_secs_f64());
    }

    fn handle_checkin(&self, event: r2d2::event::CheckinEvent) {
        DB_NUM_CHECKINS.inc();
        DB_CHECKOUT_DURATION_SUM.add(event.duration().as_secs_f64());
    }
}

lazy_static! {
    static ref DB: Db = Db::new();
}

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub fn run_db_migrations() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Running database migrations");
    let mut conn = DB.pool.get()?;
    conn.run_pending_migrations(MIGRATIONS)?;
    info!("Finished running migrations");
    Ok(())
}

pub async fn serve_db_events(opts: NotificationListeningOpts) -> Result<(), DbEventsError> {
    DB.events.run(opts).await
}

pub fn pool() -> &'static DbPool {
    &DB.pool
}

pub fn db_events() -> DbEventReceiver {
    DB.events.subscribe()
}
