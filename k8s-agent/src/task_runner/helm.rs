use super::values::create_values_and_secrets;
use crate::{
    config::Config,
    k8s::{pods::execute_pod, tracker::K8S_TRACKER},
};
use anyhow::Result;
use base64::prelude::*;
use k8s_openapi::{
    api::core::v1::{Container, EnvVar, Pod, PodSpec},
    apimachinery::pkg::apis::meta::v1::ObjectMeta,
};
use platz_db::schema::{
    deployment::Deployment, deployment_task::DeploymentTask, helm_registry::HelmRegistry,
};
use tracing::debug;

// -------------------------------------------------------------------------
// Runs helm in a pod in the current cluster. We do this, instead of running
// the helm pod in the target cluster, because we need the service account
// with permissions to the remote cluster.
// Helm will run with a kubeconfig containing only the target cluster.
// -------------------------------------------------------------------------

#[tracing::instrument(err, skip_all)]
pub async fn run_helm(
    config: &Config,
    command: &str,
    deployment: &Deployment,
    task: &DeploymentTask,
) -> Result<String> {
    debug!("cmd={command}");
    debug!("creating values and secrets...");
    let values = create_values_and_secrets(deployment, task, &config.platz_url).await?;

    execute_pod(
        &config.self_namespace,
        helm_pod(config, command, task, deployment, values).await?,
    )
    .await
}

async fn helm_pod(
    config: &Config,
    command: &str,
    task: &DeploymentTask,
    deployment: &Deployment,
    values: serde_json::Value,
) -> Result<Pod> {
    let namespace_name = deployment.namespace_name().await;

    let cluster = K8S_TRACKER.get_cluster(deployment.cluster_id).await?;
    let kubeconfig = cluster.base64_kubeconfig()?;

    let chart = task.helm_chart().await?;
    let registry = HelmRegistry::find(chart.helm_registry_id).await?;

    let script = [
        "mkdir -p /root/.kube",
        "echo $KUBECONFIG_BASE64 | base64 -d > /root/.kube/config",
        "chmod 400 /root/.kube/config",
        "aws ecr get-login-password --region $HELM_REGISTRY_REGION | helm registry login --username AWS --password-stdin $HELM_REGISTRY",
        "helm pull oci://$HELM_REGISTRY/$HELM_REPO --version $HELM_CHART_TAG",
        "echo $VALUES_BASE64 | base64 -d > values.yaml",
        "echo $VALUES_OVERRIDE_BASE64 | base64 -d > values-override.yaml",
        &format!(
            "helm --debug --kubeconfig=/root/.kube/config {command} {namespace_name} oci://$HELM_REGISTRY/$HELM_REPO --version $HELM_CHART_TAG --namespace={namespace_name} -f values.yaml -f values-override.yaml",
        ),
    ].join(" && ");

    Ok(Pod {
        metadata: ObjectMeta {
            name: Some(format!("task-{}", task.id)),
            namespace: Some(config.self_namespace.to_owned()),
            ..Default::default()
        },
        spec: Some(PodSpec {
            service_account_name: Some(config.self_service_account_name.to_owned()),
            containers: vec![Container {
                name: format!("task-{}", task.id),
                image: Some(config.helm_image.to_owned()),
                image_pull_policy: Some("Always".into()),
                command: Some(vec!["/bin/bash".into(), "-cex".into(), script]),
                env: Some(vec![
                    EnvVar {
                        name: "KUBECONFIG_BASE64".into(),
                        value: Some(kubeconfig),
                        ..Default::default()
                    },
                    EnvVar {
                        name: "HELM_REGISTRY_REGION".into(),
                        value: Some(registry.region_name()?),
                        ..Default::default()
                    },
                    EnvVar {
                        name: "HELM_REGISTRY".into(),
                        value: Some(registry.domain_name),
                        ..Default::default()
                    },
                    EnvVar {
                        name: "HELM_REPO".into(),
                        value: Some(registry.repo_name),
                        ..Default::default()
                    },
                    EnvVar {
                        name: "HELM_CHART_TAG".into(),
                        value: Some(chart.image_tag),
                        ..Default::default()
                    },
                    EnvVar {
                        name: "VALUES_BASE64".into(),
                        value: Some(BASE64_STANDARD.encode(serde_yaml::to_string(&values)?)),
                        ..Default::default()
                    },
                    EnvVar {
                        name: "VALUES_OVERRIDE_BASE64".into(),
                        value: if let Some(values_override) = &deployment.values_override {
                            Some(BASE64_STANDARD.encode(serde_yaml::to_string(values_override)?))
                        } else {
                            None
                        },
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            }],
            restart_policy: Some("Never".into()),
            ..Default::default()
        }),
        ..Default::default()
    })
}
