use anyhow::Result;
use clap::Parser;
use tokio::select;

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
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::parse();
    env_logger::Builder::new()
        .filter(None, log::LevelFilter::Debug)
        .init();

    kind::update_all_registries().await?;

    select! {
        result = ecr_events::run(&config.ecr_events) => {
            result
        }
        result = tag_parser::run() => {
            result
        }
    }
}
