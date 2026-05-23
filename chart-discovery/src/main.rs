use anyhow::{Result, anyhow};
use clap::{Parser, ValueEnum};
use platz_db::{DbTable, NotificationListeningOpts, init_db};
use tokio::{
    select,
    signal::unix::{SignalKind, signal},
};
use tracing::{info, warn};

mod charts;
mod ecr_events;
mod kind;
mod oci_poll;
mod registries;
mod sqs;
mod tag_parser;

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
#[clap(rename_all = "lowercase")]
enum RegistryProvider {
    /// Watch an SQS queue fed by ECR push/delete events. Requires AWS credentials.
    Ecr,
    /// Periodically poll a generic OCI registry (e.g. Docker Distribution / zot)
    /// for new chart artifacts. Used by local dev and air-gapped setups.
    Oci,
}

#[derive(Debug, Parser)]
pub struct Config {
    #[clap(long, env = "PLATZ_REGISTRY_PROVIDER", value_enum, default_value = "ecr")]
    provider: RegistryProvider,

    #[clap(flatten)]
    ecr_events: ecr_events::Config,

    #[clap(flatten)]
    oci_poll: oci_poll::Config,

    #[clap(long, default_value_t = false)]
    enable_tag_parser: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    platz_otel::init()?;
    let config = Config::parse();
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;

    let db = init_db().await?;

    let tag_parser_fut = async {
        if config.enable_tag_parser {
            tag_parser::run(db).await
        } else {
            futures::future::pending::<Result<()>>().await
        }
    };

    let provider_fut = async {
        match config.provider {
            RegistryProvider::Ecr => {
                let ecr_config = config
                    .ecr_events
                    .resolved()
                    .ok_or_else(|| anyhow!(
                        "PLATZ_ECR_EVENTS_QUEUE and PLATZ_ECR_EVENTS_REGION must be set when provider=ecr"
                    ))?;
                ecr_events::run(&ecr_config).await
            }
            RegistryProvider::Oci => oci_poll::run(&config.oci_poll).await,
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

        result = provider_fut => {
            result
        }

        result = tag_parser_fut => {
            result
        }
    }
}
