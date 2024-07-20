use crate::k8s::K8S_TRACKER;
use anyhow::{Context, Result};
use k8s_openapi::api::core::v1::Secret;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::api::{Api, Patch, PatchParams};
use platz_chart_ext::UiSchema;
use platz_db::{DbTableOrDeploymentResource, Deployment, DeploymentTask};
use std::collections::BTreeMap;
use tracing::debug;
use uuid::Uuid;

pub async fn apply_secrets(
    env_id: Uuid,
    ui_schema: &UiSchema,
    deployment: &Deployment,
    task: &DeploymentTask,
) -> Result<()> {
    let inputs = task.get_config()?;
    for secret in ui_schema
        .get_secrets::<DbTableOrDeploymentResource>(env_id, inputs)
        .await?
        .into_iter()
    {
        apply_secret(
            deployment.cluster_id,
            &deployment.namespace_name(),
            &secret.name,
            secret.attrs,
        )
        .await?;
    }
    Ok(())
}

#[tracing::instrument(err, skip_all, fields(%cluster_id, %namespace, %name))]
pub async fn apply_secret(
    cluster_id: Uuid,
    namespace: &str,
    name: &str,
    attrs: BTreeMap<String, String>,
) -> Result<()> {
    let api = Api::<Secret>::namespaced(
        K8S_TRACKER
            .get_cluster(cluster_id)
            .await?
            .kube_client()
            .await?,
        namespace,
    );

    let secret = Secret {
        metadata: ObjectMeta {
            name: Some(name.to_string()),
            ..Default::default()
        },
        type_: Some("Opaque".to_owned()),
        string_data: Some(attrs),
        ..Default::default()
    };

    let params = PatchParams::apply(name);
    let patch = Patch::Apply(&secret);
    debug!("applying...");
    api.patch(name, &params, &patch)
        .await
        .context("Failed applying secrets")?;
    Ok(())
}
