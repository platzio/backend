mod events;
mod status_config;
mod tracker;

use crate::tracker::StatusTracker;
use anyhow::Result;
use log::*;
use clap::Parser;

#[derive( Debug,Parser)]
pub struct Config {
    /// Turn debug logs on
    #[clap(long)]
    debug: bool,

    /// Turn debug logs for all crates (not recommended)
    #[clap(long)]
    all_debug: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self::parse()
    }
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
    let config = Config::default();
    env_logger::Builder::new()
        .filter(Some(env!("CARGO_PKG_NAME")), config.log_level())
        .filter(None, config.all_log_level())
        .init();

    info!("Starting status updates worker");

    platz_db::init_db(false).await?;
    events::watch_deployments(StatusTracker::new()).await
}
