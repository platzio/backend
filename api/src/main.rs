use clap::Parser;
use log::*;
use platz_db::init_db;

mod permissions;
mod result;
mod routes;
mod serde_utils;
mod server;

#[derive(Parser)]
struct Config {
    #[clap(flatten)]
    server: server::Config,

    #[clap(flatten)]
    auth: platz_auth::Config,

    #[clap(long, default_value = "5secs")]
    prometheus_update_interval: humantime::Duration,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let config = Config::parse();
    env_logger::Builder::new()
        .filter(None, log::LevelFilter::Debug)
        .init();

    init_db(true).await?;

    let oidc_login = config.auth.into();

    tokio::select! {
        result = crate::routes::metrics::update_metrics_task(
            config.prometheus_update_interval.into(),
            ) => {
            warn!("Prometheus metrics finished");
            result
        }

        result = server::serve(config.server, oidc_login) => {
            warn!("API server finished");
            result
        }
    }
}
