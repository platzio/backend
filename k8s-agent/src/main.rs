mod config;
mod k8s;
mod task_runner;

use crate::config::CONFIG;
use anyhow::Result;
use log::*;

pub async fn _main() -> Result<()> {
    platz_db::init_db(false).await?;

    tokio::select! {
        result = k8s::scan_for_new_clusters(CONFIG.k8s_refresh_interval()) => {
            warn!("EKS discovery task finished");
            result?;
        }

        result = task_runner::start() => {
            warn!("Task runner finished");
            result?;
        }
    };

    Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::Builder::new()
        .filter(Some(env!("CARGO_PKG_NAME")), CONFIG.log_level())
        .filter(None, CONFIG.all_log_level())
        .init();

    info!("Starting K8S worker");

    if let Err(e) = _main().await {
        error!("{:?}", e);
        std::process::exit(1);
    }
}
