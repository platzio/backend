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
    /// Turn debug logs on
    #[clap(long)]
    debug: bool,

    /// Turn debug logs for all crates (not recommended)
    #[clap(long)]
    all_debug: bool,

    #[clap(flatten)]
    ecr_events: ecr_events::Config,
}

impl Config {
    pub fn log_level(&self) -> log::LevelFilter {
        match self.debug {
            true => log::LevelFilter::Debug,
            false => log::LevelFilter::Info,
        }
    }

    pub fn all_log_level(&self) -> log::LevelFilter {
        match self.all_debug {
            true => log::LevelFilter::Debug,
            false => log::LevelFilter::Info,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::parse();
    env_logger::Builder::new()
        .filter(Some(env!("CARGO_PKG_NAME")), config.log_level())
        .filter(None, config.all_log_level())
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
