use clap::Parser;
use log::*;
use platz_db::init_db;

mod permissions;
mod result;
mod routes;
mod serde_utils;
mod server;

#[derive(Parser)]
enum Command {
    #[command(name = "run")]
    Run {
        #[clap(flatten)]
        server_config: server::Config,
        #[clap(flatten)]
        auth_config: platz_auth::Config,
        #[clap(long, default_value = "5secs")]
        prometheus_update_interval: humantime::Duration,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let command = Command::parse();

    match command {
        Command::Run {
            server_config,
            auth_config,
            prometheus_update_interval,
        } => {
            env_logger::Builder::new()
                .filter(None, log::LevelFilter::Debug)
                .init();

            init_db(true).await?;

            let oidc_login = auth_config.into();

            tokio::select! {
                result = crate::routes::metrics::update_metrics_task(
                    prometheus_update_interval.into(),
                    ) => {
                    warn!("Prometheus metrics finished");
                    result
                }

                result = server::serve(server_config, oidc_login) => {
                    warn!("API server finished");
                    result
                }
            }
        }
    }
}
