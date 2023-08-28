use clap::{Parser, Subcommand};
use log::*;
use platz_db::init_db;
use routes::openapi::SchemaFormat;

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
    #[command(subcommand)]
    Openapi(OpenapiSubcommand),
}

#[derive(Subcommand)]
#[command(name = "openapi")]
enum OpenapiSubcommand {
    #[command(name = "schema")]
    Schema {
        #[arg(default_value_t = SchemaFormat::Yaml)]
        format: SchemaFormat,
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
            tracing_subscriber::fmt::init();

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
        Command::Openapi(OpenapiSubcommand::Schema { format }) => {
            let schema = routes::openapi::get_schema(format);
            println!("{}", schema);
            Ok(())
        }
    }
}
