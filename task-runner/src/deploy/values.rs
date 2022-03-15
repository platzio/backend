use super::secrets::apply_secrets;
use crate::k8s::K8S_TRACKER;
use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use platz_chart_ext::insert_into_map;
use platz_db::{DbTable, Deployment, DeploymentTask, Env, K8sCluster};
use serde::Serialize;
use std::env;
use url::Url;
use uuid::Uuid;

lazy_static! {
    static ref OWN_URL: Url = Url::parse(
        &env::var("PLATZ_OWN_URL").expect("PLATZ_OWN_URL environment variable is not defined")
    )
    .unwrap();
}

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
    cluster_id: Uuid,
    cluster_name: String,
    deployment_id: Uuid,
    deployment_name: String,
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
    fn new(host: String) -> Self {
        Self {
            enabled: true,
            hosts: vec![IngressHost::new(host.clone())],
            tls: vec![IngressTls::new(host)],
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
    fn new(host: String) -> Self {
        Self {
            secret_name: "tls-wildcard".to_owned(),
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
    let ui_schema = chart.values_ui();
    let features = chart
        .features()
        .map_err(|err| anyhow!("Error parsing chart features: {}", err))?;

    let platz_info = PlatzInfo {
        cluster_id: deployment.cluster_id,
        cluster_name: cluster.name()?.to_owned(),
        deployment_id: deployment.id,
        deployment_name: deployment.name.to_owned(),
        revision_id: task.id,
        own_url: OWN_URL.to_owned(),
    };

    let mut values = serde_json::to_value(ChartValues {
        shira: platz_info.clone(),
        platz: platz_info,
        node_selector: env.node_selector.clone(),
        tolerations: env.tolerations.clone(),
        ingress: if features.standard_ingress() {
            let ingress_host = deployment.standard_ingress_hostname().await?;
            Ingress::new(ingress_host)
        } else {
            Default::default()
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
        let mut more_values = ui_schema.get_values::<DbTable>(env.id, inputs).await?;
        values.as_object_mut().unwrap().append(&mut more_values);
        apply_secrets(env.id, ui_schema, deployment, task).await?;
    } else {
        values
            .as_object_mut()
            .unwrap()
            .insert("config".to_owned(), deployment.config.clone());
    }

    Ok(values)
}
