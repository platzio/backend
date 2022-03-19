use anyhow::Result;
use clap::Parser;
use log::*;
use tokio::time;

mod charts;
mod ecr_events;
mod registries;

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
async fn main() {
    let config = Config::parse();
    env_logger::Builder::new()
        .filter(Some(env!("CARGO_PKG_NAME")), config.log_level())
        .filter(None, config.all_log_level())
        .init();

    info!("Starting to watch for ECR events");

    if let Err(e) = _main(&config).await {
        error!("{:?}", e);
        std::process::exit(1);
    }
}

async fn _main(config: &Config) -> Result<()> {
    loop {
        ecr_events::run(&config.ecr_events).await?;
        time::sleep(time::Duration::from_secs(10)).await;
    }
}
