use anyhow::Result;
use clap::Parser;
use platz_db::{init_db, DbTable, NotificationListeningOpts};
use tokio::{
    select,
    signal::unix::{signal, SignalKind},
};
use tracing::{info, warn};

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
    platz_otel::init()?;
    let config = Config::parse();
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;

    let db = init_db().await;

    let tag_parser_fut = async {
        if config.enable_tag_parser {
            tag_parser::run(db).await
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

        result = db.serve_db_events(
            NotificationListeningOpts::on_table(DbTable::HelmTagFormats),
        ) => {
            warn!("DB events task exited: {result:?}");
            result.map_err(Into::into)
        }

        result = ecr_events::run(&config.ecr_events) => {
            result
        }

        result = tag_parser_fut => {
            result
        }
    }
}
