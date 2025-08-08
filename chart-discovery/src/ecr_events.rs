use crate::registries::find_and_save_ecr_repo;
use anyhow::{anyhow, Result};
use aws_sdk_ecr::types::{ImageDetail, ImageIdentifier};
use aws_types::region::Region;
use chrono::prelude::*;
use clap::Parser;
use platz_db::schema::helm_registry::HelmRegistry;
use serde::Deserialize;
use tracing::{debug, info, warn};
use uuid::Uuid;

#[derive(Debug, Parser)]
#[group(skip)]
pub struct Config {
    #[clap(long, env = "PLATZ_ECR_EVENTS_QUEUE")]
    ecr_events_queue: String,

    #[clap(long, env = "PLATZ_ECR_EVENTS_REGION")]
    ecr_events_region: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct EcrEvent {
    pub version: String,
    pub id: Uuid,
    #[serde(rename = "detail-type")]
    pub detail_type: String,
    pub source: String,
    pub account: String,
    pub time: DateTime<Utc>,
    pub region: String,
    pub resources: Vec<String>,
    pub detail: EcrEventDetail,
}

impl EcrEvent {
    pub fn helm_registry_domain_name(&self) -> String {
        format!("{}.dkr.ecr.{}.amazonaws.com", self.account, self.region)
    }

    pub async fn find_ecr_repo(&self) -> Result<Option<HelmRegistry>> {
        let domain_name = self.helm_registry_domain_name();
        let repo_name = self.detail.repository_name.clone();
        Ok(HelmRegistry::find_by_domain_and_repo(domain_name, repo_name).await?)
    }

    pub async fn find_or_create_ecr_repo(&self) -> Result<HelmRegistry> {
        Ok(match self.find_ecr_repo().await? {
            Some(registry) => registry,
            None => {
                let region = Region::new(self.region.to_owned());
                find_and_save_ecr_repo(region, &self.detail.repository_name).await?
            }
        })
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub enum EcrEventResult {
    #[serde(rename = "SUCCESS")]
    Success,
    #[serde(rename = "FAILURE")]
    Failure,
}

#[derive(Debug, Deserialize)]
pub enum EcrActionType {
    #[serde(rename = "PUSH")]
    Push,
    #[serde(rename = "DELETE")]
    Delete,
}

#[derive(Debug, Deserialize)]
pub struct EcrEventDetail {
    pub result: EcrEventResult,
    #[serde(rename = "repository-name")]
    pub repository_name: String,
    #[serde(rename = "image-digest")]
    pub image_digest: String,
    #[serde(rename = "action-type")]
    pub action_type: EcrActionType,
    #[serde(rename = "image-tag")]
    pub image_tag: String,
}

impl EcrEventDetail {
    pub async fn image_detail(&self, ecr: &aws_sdk_ecr::Client) -> Result<Option<ImageDetail>> {
        let res = ecr
            .describe_images()
            .repository_name(self.repository_name.clone())
            .image_ids(
                ImageIdentifier::builder()
                    .image_digest(self.image_digest.clone())
                    .image_tag(self.image_tag.clone())
                    .build(),
            )
            .send()
            .await;
        let res = match res {
            Ok(res) => res,
            Err(aws_sdk_ecr::error::SdkError::ServiceError(service_error))
                if service_error.err().is_image_not_found_exception() =>
            {
                return Ok(None);
            }
            Err(err) => return Err(err.into()),
        };
        match res.image_details {
            None => Err(anyhow!("Failed getting image detail: {:?}", self)),
            Some(v) => {
                if v.len() == 1 {
                    Ok(Some(v.into_iter().next().unwrap()))
                } else {
                    Err(anyhow!("Expected exactly one result, got: {:?}", v))
                }
            }
        }
    }
}

async fn handle_ecr_event(ecr: &aws_sdk_ecr::Client, event: EcrEvent) -> Result<()> {
    debug!("Event: {:?}", event);

    if event.detail.result == EcrEventResult::Failure {
        warn!("Ignoring failure event");
        return Ok(());
    }

    match event.detail.action_type {
        EcrActionType::Push => crate::charts::add_helm_chart(ecr, event).await,
        EcrActionType::Delete => crate::charts::discard_helm_chart(event).await,
    }
}

pub async fn run(config: &Config) -> Result<()> {
    info!("Starting to watch for ECR events");
    let region = Region::new(config.ecr_events_region.to_owned());

    let shared_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let ecr_config = aws_sdk_ecr::config::Builder::from(&shared_config)
        .region(region.clone())
        .build();
    let ecr = aws_sdk_ecr::Client::from_conf(ecr_config);

    crate::sqs::handle_messages(region, &config.ecr_events_queue, |event| {
        handle_ecr_event(&ecr, event)
    })
    .await
}
