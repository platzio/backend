use anyhow::Result;
use log::*;
use structopt::StructOpt;
use tokio::time;

mod charts;
mod ecr_events;
mod registries;

#[derive(StructOpt, Debug)]
pub struct Config {
    /// Turn debug logs on
    #[structopt(long)]
    debug: bool,

    /// Turn debug logs for all crates (not recommended)
    #[structopt(long)]
    all_debug: bool,

    #[structopt(flatten)]
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
    let config = Config::from_args();
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
