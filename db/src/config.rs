use std::env;
use std::str::FromStr;
use std::time::Duration;

const DB_POOL_MAX_SIZE: &str = "DB_POOL_MAX_SIZE";
const DB_POOL_MIN_IDLE: &str = "DB_POOL_MIN_IDLE";
const DB_POOL_CONNECTION_TIMEOUT_SECS: &str = "DB_POOL_CONNECTION_TIMEOUT_SECS";
const DB_POOL_IDLE_TIMEOUT_SECS: &str = "DB_POOL_IDLE_TIMEOUT_SECS";
const DB_POOL_MAX_LIFETIME_SECS: &str = "DB_POOL_MAX_LIFETIME_SECS";

const PGSSLMODE: &str = "PGSSLMODE";
const PGSSLROOTCERT: &str = "PGSSLROOTCERT";

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

/// How the backend negotiates TLS when connecting to PostgreSQL.
///
/// Mirrors the relevant subset of libpq's `sslmode` values. Because Platz
/// assembles the connection itself (it does not link libpq), this is the
/// authoritative knob — `PGSSLMODE` and `PGSSLROOTCERT` are read here and
/// applied to both the connection pool and the `LISTEN`/`NOTIFY` connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SslMode {
    /// Never use TLS. The connection is plaintext (legacy behavior).
    Disable,
    /// Try TLS first, fall back to plaintext if the server doesn't offer it.
    /// The server certificate is not verified.
    Prefer,
    /// Require TLS, but do not verify the server certificate.
    Require,
    /// Require TLS and verify the server certificate chain against the
    /// trusted CAs, including that the hostname matches the certificate.
    VerifyFull,
}

impl std::str::FromStr for SslMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "disable" => Ok(Self::Disable),
            "prefer" => Ok(Self::Prefer),
            "require" => Ok(Self::Require),
            "verify-full" | "verify_full" => Ok(Self::VerifyFull),
            other => Err(format!(
                "Invalid {PGSSLMODE} value {other:?}. \
                 Expected one of: disable, prefer, require, verify-full"
            )),
        }
    }
}

/// Resolved TLS settings for connecting to PostgreSQL, derived from the
/// `PGSSLMODE` and `PGSSLROOTCERT` environment variables.
#[derive(Debug, Clone)]
pub struct SslSettings {
    pub mode: SslMode,
    /// Path to a PEM-encoded CA bundle used to verify the server certificate
    /// in `verify-full` mode. When `None`, the system trust store is used.
    pub root_cert: Option<String>,
}

impl SslSettings {
    /// Reads the TLS settings from the environment.
    ///
    /// `PGSSLMODE` defaults to `prefer` (opportunistic TLS) so that existing
    /// plaintext databases keep working while TLS-capable databases are used
    /// encrypted without any extra configuration.
    pub fn from_env() -> Result<Self, String> {
        let mode = match env::var(PGSSLMODE) {
            Ok(value) => value.parse()?,
            Err(_) => SslMode::Prefer,
        };
        let root_cert = env::var(PGSSLROOTCERT).ok().filter(|s| !s.is_empty());
        Ok(Self { mode, root_cert })
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_ssl_modes() {
        assert_eq!("disable".parse::<SslMode>().unwrap(), SslMode::Disable);
        assert_eq!("prefer".parse::<SslMode>().unwrap(), SslMode::Prefer);
        assert_eq!("require".parse::<SslMode>().unwrap(), SslMode::Require);
        assert_eq!(
            "verify-full".parse::<SslMode>().unwrap(),
            SslMode::VerifyFull
        );
    }

    #[test]
    fn ssl_mode_parsing_is_case_and_whitespace_insensitive() {
        assert_eq!(
            "  VERIFY_FULL ".parse::<SslMode>().unwrap(),
            SslMode::VerifyFull
        );
        assert_eq!("Require".parse::<SslMode>().unwrap(), SslMode::Require);
    }

    #[test]
    fn rejects_unknown_ssl_mode() {
        assert!("verify-ca".parse::<SslMode>().is_err());
        assert!("".parse::<SslMode>().is_err());
    }
}
