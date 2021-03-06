use super::secrets::apply_secrets;
use crate::config::OWN_URL;
use crate::k8s::K8S_TRACKER;
use anyhow::{anyhow, Result};
use log::*;
use platz_chart_ext::{insert_into_map, UiSchema};
use platz_db::{DbTableOrDeploymentResource, Deployment, DeploymentTask, Env, K8sCluster};
use serde::Serialize;
use url::Url;
use uuid::Uuid;

#[derive(Serialize)]
struct ChartValues {
    platz: PlatzInfo,
    shira: PlatzInfo, // Support old name
    #[serde(rename = "nodeSelector")]
    node_selector: serde_json::Value,
    tolerations: serde_json::Value,
    ingress: Ingress,
}

#[derive(Clone, Serialize)]
struct PlatzInfo {
    env_id: Uuid,
    env_name: String,
    cluster_id: Uuid,
    cluster_name: String,
    deployment_id: Uuid,
    deployment_name: String,
    deployment_kind: String,
    revision_id: Uuid,
    own_url: Url,
}

#[derive(Default, Serialize)]
struct Ingress {
    enabled: bool,
    hosts: Vec<IngressHost>,
    tls: Vec<IngressTls>,
}

impl Ingress {
    fn new(host: String, secret_name: String) -> Self {
        Self {
            enabled: true,
            hosts: vec![IngressHost::new(host.clone())],
            tls: vec![IngressTls::new(host, secret_name)],
        }
    }
}

#[derive(Serialize)]
struct IngressHost {
    host: String,
    paths: Vec<IngressHostPath>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IngressHostPath {
    path: String,
    path_type: String,
}

impl IngressHost {
    fn new(host: String) -> Self {
        Self {
            host,
            paths: vec![IngressHostPath {
                path: "/".to_owned(),
                path_type: "Prefix".to_owned(),
            }],
        }
    }
}

#[derive(Serialize)]
struct IngressTls {
    #[serde(rename = "secretName")]
    secret_name: String,
    hosts: Vec<String>,
}

impl IngressTls {
    fn new(host: String, secret_name: String) -> Self {
        Self {
            secret_name,
            hosts: vec![host],
        }
    }
}

pub async fn create_values_and_secrets(
    deployment: &Deployment,
    task: &DeploymentTask,
) -> Result<serde_json::Value> {
    let cluster = K8S_TRACKER.get_cluster(deployment.cluster_id).await?;
    let db_cluster = K8sCluster::find(deployment.cluster_id).await?;
    let env = Env::find(
        db_cluster
            .env_id
            .ok_or_else(|| anyhow!("Could not find cluster for deployment"))?,
    )
    .await?;
    let chart = task.helm_chart().await?;
    let ui_schema: Option<UiSchema> = chart
        .values_ui
        .clone()
        .map(serde_json::from_value)
        .transpose()?;
    let features = chart
        .features()
        .map_err(|err| anyhow!("Error parsing chart features: {}", err))?;

    let platz_info = PlatzInfo {
        env_id: env.id,
        env_name: env.name,
        cluster_id: deployment.cluster_id,
        cluster_name: cluster.name()?.to_owned(),
        deployment_id: deployment.id,
        deployment_name: deployment.name.to_owned(),
        deployment_kind: deployment.kind.to_owned(),
        revision_id: task.id,
        own_url: OWN_URL.to_owned(),
    };

    let mut values = serde_json::to_value(ChartValues {
        shira: platz_info.clone(),
        platz: platz_info,
        node_selector: env.node_selector.clone(),
        tolerations: env.tolerations.clone(),
        ingress: {
            let ingress = features.ingress();
            if ingress.enabled {
                if let Some(secret_name) = db_cluster.domain_tls_secret_name {
                    let ingress_host = deployment.ingress_hostname(ingress.hostname_format).await?;
                    Ingress::new(ingress_host, secret_name.clone())
                } else {
                    warn!("Deployment standard_ingress is enabled but domain_tls_secret_name is not configured for the cluster. Not creating ingress.");
                    Default::default()
                }
            } else {
                Default::default()
            }
        },
    })?;

    for path in features.node_selector_paths().iter() {
        insert_into_map(
            values.as_object_mut().unwrap(),
            path,
            env.node_selector.clone(),
        );
    }

    for path in features.tolerations_paths().iter() {
        insert_into_map(
            values.as_object_mut().unwrap(),
            path,
            env.tolerations.clone(),
        );
    }

    if let Some(ui_schema) = ui_schema {
        let inputs = task.get_config()?;
        let mut more_values = ui_schema
            .get_values::<DbTableOrDeploymentResource>(env.id, inputs)
            .await?;
        values.as_object_mut().unwrap().append(&mut more_values);
        apply_secrets(env.id, &ui_schema, deployment, task).await?;
    } else {
        values
            .as_object_mut()
            .unwrap()
            .insert("config".to_owned(), deployment.config.clone());
    }

    Ok(values)
}
