use clap::Parser;
use std::env;

#[derive(Parser)]
struct DatabaseConfig {
    #[arg(env = "PGHOST")]
    pg_host: String,
    #[arg(env = "PGPORT")]
    pg_port: u16,
    #[arg(env = "PGUSER")]
    pg_user: String,
    #[arg(env = "PGPASSWORD", hide_env_values = true)]
    pg_password: String,
    #[arg(env = "PGDATABASE")]
    pg_database: String,
}

pub fn database_url() -> String {
    // Legacy database URL, will be removed in a future version
    if let Ok(url) = env::var("DATABASE_URL") {
        return url;
    }

    let DatabaseConfig {
        pg_host,
        pg_port,
        pg_user,
        pg_password,
        pg_database,
    } = DatabaseConfig::parse();
    format!("postgres://{pg_user}:{pg_password}@{pg_host}:{pg_port}/{pg_database}")
}
