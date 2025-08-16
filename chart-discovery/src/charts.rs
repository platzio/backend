use crate::ecr_events::{EcrEvent, EcrEventDetail};
use crate::tag_parser::parse_image_tag;
use anyhow::{Result, anyhow};
use aws_smithy_types_convert::date_time::DateTimeExt;
use platz_chart_ext::ChartExt;
use platz_db::{
    Json,
    schema::helm_chart::{HelmChart, HelmChartTagInfo, NewHelmChart, UpdateHelmChart},
};
use std::path::PathBuf;
use tokio::process::Command;
use tracing::{debug, info, warn};

const HELM_ARTIFACT_MEDIA_TYPE: &str = "application/vnd.cncf.helm.config.v1+json";
const TEMP_DOWNLOAD_PATH: &str = "/tmp/platz-chart-download";

pub async fn add_helm_chart(ecr: &aws_sdk_ecr::Client, event: EcrEvent) -> Result<()> {
    if event.detail.image_tag.is_empty() {
        warn!("The event has no image_tag, skipping: {:?}", event);
        return Ok(());
    }

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

    let created_at = image_detail
        .image_pushed_at
        .ok_or_else(|| {
            anyhow!(
                "ECR image detail has no \"image_pushed_at\": {:?}",
                image_detail
            )
        })?
        .to_chrono_utc()?;

    let helm_registry = event.find_or_create_ecr_repo().await?;

    let chart_ext = match download_chart(&event).await {
        Ok(path) => ChartExt::from_path(&path).await?,
        Err(err) => ChartExt::new_with_error(err.to_string()),
    };

    let tag_info = match chart_ext.metadata {
        Some(metadata) if metadata.git_commit.is_none() && metadata.git_branch.is_none() => {
            let parsed = parse_image_tag(&event.detail.image_tag).await?;
            HelmChartTagInfo {
                tag_format_id: parsed.tag_format_id,
                parsed_commit: parsed.parsed_commit,
                parsed_branch: parsed.parsed_branch,
                parsed_version: Some(metadata.version),
                parsed_revision: None,
            }
        }
        Some(metadata) => HelmChartTagInfo {
            tag_format_id: None,
            parsed_commit: metadata.git_commit,
            parsed_branch: metadata.git_branch,
            parsed_version: Some(metadata.version),
            parsed_revision: None,
        },
        None => parse_image_tag(&event.detail.image_tag).await?,
    };

    let chart = NewHelmChart {
        created_at,
        helm_registry_id: helm_registry.id,
        image_digest: event.detail.image_digest,
        image_tag: event.detail.image_tag,
        values_ui: chart_ext.ui_schema.map(Json),
        actions_schema: chart_ext.actions.map(Json),
        features: chart_ext.features.map(Json),
        resource_types: chart_ext.resource_types.map(Json),
        error: chart_ext.error,
        tag_format_id: tag_info.tag_format_id,
        parsed_version: tag_info.parsed_version,
        parsed_revision: tag_info.parsed_revision,
        parsed_branch: tag_info.parsed_branch,
        parsed_commit: tag_info.parsed_commit,
    }
    .insert()
    .await?;

    info!("Added helm chart {:?}", chart);
    Ok(())
}

async fn download_chart(event: &EcrEvent) -> Result<PathBuf> {
    info!("Downloading chart");
    let path = PathBuf::from(TEMP_DOWNLOAD_PATH);
    let script = [
        "rm -rf $TEMP_DOWNLOAD_PATH",
        "mkdir -p $TEMP_DOWNLOAD_PATH",
        "cd $TEMP_DOWNLOAD_PATH",
        "aws ecr get-login-password --region $HELM_REGISTRY_REGION | helm registry login --username AWS --password-stdin $HELM_REGISTRY",
        "helm pull oci://$HELM_REGISTRY/$HELM_REPO --version $HELM_CHART_TAG -d ./ --untar",
    ].join(" && ");

    let output = Command::new("/usr/bin/bash")
        .arg("-euxc")
        .arg(&script)
        .env("TEMP_DOWNLOAD_PATH", TEMP_DOWNLOAD_PATH)
        .env("HELM_REGISTRY_REGION", &event.region)
        .env("HELM_REGISTRY", event.helm_registry_domain_name())
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
            UpdateHelmChart { available: false }.save(chart.id).await?;
        }
    }

    Ok(())
}
