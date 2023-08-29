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

    #[clap(long, default_value_t = false)]
    enable_tag_parser: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::parse();

    platz_db::init_db(false).await?;
    kind::update_all_registries().await?;

    let tag_parser_fut = async {
        if config.enable_tag_parser {
            tag_parser::run().await
        } else {
            futures::future::pending::<Result<()>>().await
        }
    };

    select! {
        result = ecr_events::run(&config.ecr_events) => {
            result
        }
        result = tag_parser_fut => {
            result
        }
    }
}
