use anyhow::Result;
use clap::Parser;
use platz_db::DbTable;
use platz_db::NotificationListeningOpts;
use tokio::select;
use tokio::signal::unix::signal;
use tokio::signal::unix::SignalKind;
use tracing::info;

mod charts;
mod ecr_events;
mod kind;
mod registries;
mod sqs;
mod tag_parser;

#[derive(Debug, Parser)]
pub struct Config {
    #[clap(flatten)]
    ecr_events: ecr_events::Config,

    #[clap(long, default_value_t = false)]
    enable_tag_parser: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let config = Config::parse();
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;

    platz_db::init_db(
        false,
        NotificationListeningOpts::on_table(DbTable::HelmTagFormats),
    )
    .await?;
    kind::update_all_registries().await?;

    let tag_parser_fut = async {
        if config.enable_tag_parser {
            tag_parser::run().await
        } else {
            futures::future::pending::<Result<()>>().await
        }
    };

    select! {
        _ = sigterm.recv() => {
            info!("SIGTERM received, exiting");
            Ok(())
        }

        _ = sigint.recv() => {
            info!("SIGINT received, exiting");
            Ok(())
        }

        result = ecr_events::run(&config.ecr_events) => {
            result
        }

        result = tag_parser_fut => {
            result
        }
    }
}
