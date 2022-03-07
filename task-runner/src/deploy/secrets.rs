use crate::k8s::K8S_TRACKER;
use anyhow::Result;
use k8s_openapi::api::core::v1::Secret;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::api::{Api, Patch, PatchParams};
use platz_db::{DbTable, Deployment, DeploymentTask};
use platz_ui_schema::UiSchema;
use std::collections::BTreeMap;
use uuid::Uuid;

pub async fn apply_secrets(
    ui_schema: &UiSchema,
    deployment: &Deployment,
    task: &DeploymentTask,
) -> Result<()> {
    let config = task.get_config()?;
    for (secret_name, attrs_schema) in ui_schema.outputs.secrets.iter() {
        let mut attrs: BTreeMap<String, String> = Default::default();
        for (key, attr_schema) in attrs_schema.iter() {
            let value = attr_schema
                .resolve::<DbTable>(&ui_schema.inputs, config)
                .await?;
            attrs.insert(
                key.clone(),
                value
                    .as_str()
                    .map_or_else(|| value.to_string(), |v| v.to_owned()),
            );
        }
        apply_secret(
            deployment.cluster_id,
            &deployment.namespace_name(),
            secret_name,
            attrs,
        )
        .await?;
    }
    Ok(())
}

async fn apply_secret(
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
    api.patch(name, &params, &patch).await?;
    Ok(())
}
