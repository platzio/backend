use crate::registries::find_and_save_ecr_repo;
use anyhow::{anyhow, Result};
use chrono::prelude::*;
use log::*;
use platz_db::HelmRegistry;
use rusoto_core::RusotoError;
use rusoto_ecr::{
    DescribeImagesError, DescribeImagesRequest, Ecr, EcrClient, ImageDetail, ImageIdentifier,
};
use rusoto_sqs::SqsClient;
use rusoto_utils::creds::rusoto_client;
use rusoto_utils::regions::Region;
use rusoto_utils::sqs::handle_sqs_messages;
use serde::Deserialize;
use std::str::FromStr;
use structopt::StructOpt;
use uuid::Uuid;

#[derive(Debug, StructOpt)]
pub struct Config {
    #[structopt(long, env = "PLATZ_ECR_EVENTS_QUEUE")]
    ecr_events_queue: String,

    #[structopt(long, env = "PLATZ_ECR_EVENTS_REGION")]
    ecr_events_region: Region,
}

#[derive(Debug, Deserialize)]
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
                let region = Region::from_str(&self.region)?;
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
    pub async fn image_detail(&self, ecr: &EcrClient) -> Result<Option<ImageDetail>> {
        let res = ecr
            .describe_images(DescribeImagesRequest {
                repository_name: self.repository_name.clone(),
                image_ids: Some(vec![ImageIdentifier {
                    image_digest: Some(self.image_digest.clone()),
                    image_tag: Some(self.image_tag.clone()),
                }]),
                ..Default::default()
            })
            .await;
        let res = match res {
            Ok(res) => res,
            Err(RusotoError::Service(DescribeImagesError::ImageNotFound(_))) => return Ok(None),
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

async fn handle_ecr_event(ecr: &EcrClient, event: EcrEvent) -> Result<()> {
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
    let client = rusoto_client(env!("CARGO_CRATE_NAME").to_owned())?;
    let ecr = EcrClient::new_with_client(client, config.ecr_events_region.clone());

    handle_sqs_messages(
        SqsClient::new_with_client(
            rusoto_client(env!("CARGO_PKG_NAME").to_owned())?,
            config.ecr_events_region.clone(),
        ),
        &config.ecr_events_queue,
        |event| handle_ecr_event(&ecr, event),
    )
    .await
}
