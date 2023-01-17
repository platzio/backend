mod config;
mod task;

use crate::config::Config;
use crate::task::{monitor_deployment_resource_changes, scrub_deployment_resources};
use anyhow::Result;
use log::*;

pub async fn _main(_config: Config) -> Result<()> {
    platz_db::init_db(false).await?;

    let fut = tokio::spawn(monitor_deployment_resource_changes());

    info!("Scrubbing all existing deployment resources");
    scrub_deployment_resources().await?;
    info!("Finished scrubbing, will now watch for changes");

    fut.await?
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::default();
    env_logger::Builder::new()
        .filter(Some(env!("CARGO_PKG_NAME")), config.log_level())
        .filter(None, config.all_log_level())
        .init();

    info!("Starting deployment resource sync worker");

    _main(config).await
}
