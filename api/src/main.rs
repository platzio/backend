use anyhow::Result;
use clap::{Parser, Subcommand};
use platz_db::{init_db, NotificationListeningOpts};
use routes::openapi::SchemaFormat;
use tokio::{
    select,
    signal::unix::{signal, SignalKind},
};
use tracing::warn;

mod permissions;
mod result;
mod routes;
mod serde_utils;
mod server;

#[derive(Parser)]
enum Command {
    #[command(name = "run")]
    Run(Box<RunCommand>),
    #[command(subcommand)]
    Openapi(OpenapiCommand),
}

#[derive(clap::Args)]
struct RunCommand {
    #[clap(flatten)]
    server_config: server::Config,
    #[clap(long, default_value = "5secs")]
    prometheus_update_interval: humantime::Duration,
}

impl RunCommand {
    async fn run(self) -> Result<()> {
        tracing_subscriber::fmt::init();

        init_db(true, NotificationListeningOpts::all()).await?;
        let mut sigterm = signal(SignalKind::terminate())?;
        let mut sigint = signal(SignalKind::interrupt())?;

        select! {
            _ = sigterm.recv() => {
                warn!("SIGTERM received, exiting");
                Ok(())
            }

            _ = sigint.recv() => {
                warn!("SIGINT received, exiting");
                Ok(())
            }

            result = crate::routes::metrics::update_metrics_task(
                self.prometheus_update_interval.into(),
                ) => {
                warn!("Prometheus metrics finished: {result:?}");
                result
            }

            result = server::serve(self.server_config) => {
                warn!("API server finished: {result:?}");
                result
            }
        }
    }
}

#[derive(Subcommand)]
#[command(name = "openapi")]
enum OpenapiCommand {
    #[command(name = "schema")]
    Schema {
        #[arg(default_value_t = SchemaFormat::Yaml)]
        format: SchemaFormat,
    },
}

impl OpenapiCommand {
    fn run(self) -> Result<()> {
        let OpenapiCommand::Schema { format } = self;
        let schema = routes::openapi::get_schema(format);
        println!("{}", schema);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let command = Command::parse();

    match command {
        Command::Run(command) => command.run().await?,
        Command::Openapi(command) => command.run()?,
    }
    warn!("done");
    Ok(())
}
