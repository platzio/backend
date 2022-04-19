use crate::ecr_events::{EcrEvent, EcrEventDetail};
use anyhow::{anyhow, Result};
use chrono::prelude::*;
use log::*;
use platz_chart_ext::ChartExt;
use platz_db::{diesel_json, HelmChart, NewHelmChart, UpdateHelmChart};
use rusoto_ecr::EcrClient;
use std::path::PathBuf;
use tokio::process::Command;

const HELM_ARTIFACT_MEDIA_TYPE: &str = "application/vnd.cncf.helm.config.v1+json";
const TEMP_DOWNLOAD_PATH: &str = "/tmp/platz-chart-download";

pub async fn add_helm_chart(ecr: &EcrClient, event: EcrEvent) -> Result<()> {
    let image_detail = match event.detail.image_detail(ecr).await? {
        Some(image_detail) => image_detail,
        None => {
            warn!("Couldn't get image detail, it was probably deleted");
            return Ok(());
        }
    };

    if image_detail.artifact_media_type != Some(HELM_ARTIFACT_MEDIA_TYPE.to_owned()) {
        warn!("Ignoring image due to artifact_media_type");
        return Ok(());
    }

    let created_at = DateTime::from_utc(
        NaiveDateTime::from_timestamp(image_detail.image_pushed_at.unwrap() as i64, 0),
        Utc,
    );

    let helm_registry = event.find_or_create_ecr_repo().await?;

    let chart_path = download_chart(&event).await?;
    let chart_ext = ChartExt::from_path(&chart_path).await?;

    let chart = NewHelmChart {
        created_at,
        helm_registry_id: helm_registry.id,
        image_digest: event.detail.image_digest,
        image_tag: event.detail.image_tag,
        values_ui: chart_ext.values_ui.map(diesel_json::Json),
        actions_schema: chart_ext.actions.map(diesel_json::Json),
        features: chart_ext.features.map(diesel_json::Json),
        resource_types: chart_ext.resource_types.map(diesel_json::Json),
        error: chart_ext.error,
    }
    .insert()
    .await?;

    info!("Added helm chart {:?}", chart);
    Ok(())
}

async fn download_chart(event: &EcrEvent) -> Result<PathBuf> {
    info!("Downloading chart");
    let path = PathBuf::from(TEMP_DOWNLOAD_PATH);
    let script = vec![
        "rm -rf $TEMP_DOWNLOAD_PATH",
        "mkdir -p $TEMP_DOWNLOAD_PATH",
        "cd $TEMP_DOWNLOAD_PATH",
        "aws ecr get-login-password --region $HELM_REGISTRY_REGION | helm registry login --username AWS --password-stdin $HELM_REGISTRY",
        "helm pull oci://$HELM_REGISTRY/$HELM_REPO --version $HELM_CHART_TAG -d ./",
    ].join(" && ");

    let output = Command::new("/usr/bin/bash")
        .arg("-euxc")
        .arg(&script)
        .env("TEMP_DOWNLOAD_PATH", TEMP_DOWNLOAD_PATH)
        .env("HELM_REGISTRY_REGION", &event.region)
        .env("HELM_REGISTRY", &event.helm_registry_domain_name())
        .env("HELM_REPO", &event.detail.repository_name)
        .env("HELM_CHART_TAG", &event.detail.image_tag)
        .spawn()?
        .wait_with_output()
        .await?;

    info!("Finished downloading chart ({:?})", output.status);
    info!("Stdout: {}", String::from_utf8_lossy(&output.stdout));
    info!("Stderr: {}", String::from_utf8_lossy(&output.stderr));

    let mut dir = tokio::fs::read_dir(path).await?;
    let mut files = Vec::new();
    while let Some(entry) = dir.next_entry().await? {
        files.push(entry.path());
    }
    match files.len() {
        1 => Ok(files.into_iter().next().unwrap()),
        _ => Err(anyhow!(
            "Expected exactly one folder after exporting chart, found: {:?}",
            files
        )),
    }
}

pub async fn discard_helm_chart(event: EcrEvent) -> Result<()> {
    let EcrEventDetail {
        image_digest,
        image_tag,
        ..
    } = &event.detail;
    info!(
        "Discarding ECR image: image_digest={}, image_tag={}",
        image_digest, image_tag
    );

    let helm_registry = match event.find_ecr_repo().await? {
        None => {
            debug!("Registry not found, ignoring this ECR image");
            return Ok(());
        }
        Some(helm_registry) => helm_registry,
    };

    match HelmChart::find_by_registry_and_digest(helm_registry.id, image_digest.to_owned()).await? {
        None => debug!("ECR image not found in database, ignoring"),
        Some(chart) => {
            UpdateHelmChart {
                available: Some(false),
            }
            .save(chart.id)
            .await?;
        }
    }

    Ok(())
}
