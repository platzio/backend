use std::env;
use std::str::FromStr;
use std::time::Duration;

const DB_POOL_MAX_SIZE: &str = "DB_POOL_MAX_SIZE";
const DB_POOL_MIN_IDLE: &str = "DB_POOL_MIN_IDLE";
const DB_POOL_CONNECTION_TIMEOUT_SECS: &str = "DB_POOL_CONNECTION_TIMEOUT_SECS";
const DB_POOL_IDLE_TIMEOUT_SECS: &str = "DB_POOL_IDLE_TIMEOUT_SECS";
const DB_POOL_MAX_LIFETIME_SECS: &str = "DB_POOL_MAX_LIFETIME_SECS";

#[derive(Debug, Clone)]
pub struct DbPoolOptions {
    pub max_size: u32,
    pub min_idle: Option<u32>,
    pub connection_timeout: Duration,
    pub idle_timeout: Option<Duration>,
    pub max_lifetime: Option<Duration>,
}

impl Default for DbPoolOptions {
    fn default() -> Self {
        Self {
            max_size: 50,
            min_idle: None,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(1800)),
        }
    }
}

pub fn database_url() -> String {
    // Legacy database URL, will be removed in a future version
    if let Ok(url) = env::var("DATABASE_URL") {
        return url;
    }

    let pg_host = env::var("PGHOST").expect("No PGHOST defined");
    let pg_port = env::var("PGPORT").expect("No PGPORT defined");
    let pg_user = env::var("PGUSER").expect("No PGUSER defined");
    let pg_password = env::var("PGPASSWORD").expect("No PGPASSWORD defined");
    let pg_database = env::var("PGDATABASE").expect("No PGDATABASE defined");
    format!("postgres://{pg_user}:{pg_password}@{pg_host}:{pg_port}/{pg_database}")
}

pub fn db_pool_options() -> DbPoolOptions {
    let defaults = DbPoolOptions::default();
    DbPoolOptions {
        max_size: env_parsed::<u32>(DB_POOL_MAX_SIZE).unwrap_or(defaults.max_size),
        min_idle: env_parsed::<u32>(DB_POOL_MIN_IDLE).or(defaults.min_idle),
        connection_timeout: Duration::from_secs(
            env_parsed::<u64>(DB_POOL_CONNECTION_TIMEOUT_SECS)
                .unwrap_or(defaults.connection_timeout.as_secs()),
        ),
        idle_timeout: env_parsed::<u64>(DB_POOL_IDLE_TIMEOUT_SECS)
            .map(Duration::from_secs)
            .or(defaults.idle_timeout),
        max_lifetime: env_parsed::<u64>(DB_POOL_MAX_LIFETIME_SECS)
            .map(Duration::from_secs)
            .or(defaults.max_lifetime),
    }
}

fn env_parsed<T: FromStr>(var: &str) -> Option<T> {
    env::var(var).ok()?.parse::<T>().ok()
}
