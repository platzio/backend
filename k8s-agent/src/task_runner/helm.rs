use super::values::create_values_and_secrets;
use crate::config::CONFIG;
use crate::k8s::execute_pod;
use crate::k8s::K8S_TRACKER;
use anyhow::Result;
use base64::prelude::*;
use k8s_openapi::api::core::v1::{Container, EnvVar, Pod, PodSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use platz_db::{Deployment, DeploymentTask, HelmRegistry};
use tracing::debug;

// -------------------------------------------------------------------------
// Runs helm in a pod in the current cluster. We do this, instead of running
// the helm pod in the target cluster, because we need the service account
// with permissions to the remote cluster.
// Helm will run with a kubeconfig containing only the target cluster.
// -------------------------------------------------------------------------

#[tracing::instrument(err, skip_all)]
pub async fn run_helm(
    command: &str,
    deployment: &Deployment,
    task: &DeploymentTask,
) -> Result<String> {
    debug!("cmd={command}");
    debug!("creating values and secrets...");
    let values = create_values_and_secrets(deployment, task).await?;

    execute_pod(helm_pod(command, task, deployment, values).await?).await
}

async fn helm_pod(
    command: &str,
    task: &DeploymentTask,
    deployment: &Deployment,
    values: serde_json::Value,
) -> Result<Pod> {
    let namespace_name = deployment.namespace_name();

    let cluster = K8S_TRACKER.get_cluster(deployment.cluster_id).await?;
    let kubeconfig = cluster.base64_kubeconfig()?;

    let chart = task.helm_chart().await?;
    let registry = HelmRegistry::find(chart.helm_registry_id).await?;

    let script = [
        "mkdir -p /root/.kube",
        "echo $KUBECONFIG_BASE64 | base64 --decode > /root/.kube/config",
        "chmod 400 /root/.kube/config",
        "aws ecr get-login-password --region $HELM_REGISTRY_REGION | helm registry login --username AWS --password-stdin $HELM_REGISTRY",
        "helm pull oci://$HELM_REGISTRY/$HELM_REPO --version $HELM_CHART_TAG",
        "echo $VALUES_BASE64 | base64 --decode > values.yaml",
        "echo $VALUES_OVERRIDE_BASE64 | base64 --decode > values-override.yaml",
        &format!(
            "helm --debug --kubeconfig=/root/.kube/config {command} {namespace_name} oci://$HELM_REGISTRY/$HELM_REPO --version $HELM_CHART_TAG --namespace={namespace_name} -f values.yaml -f values-override.yaml",
        ),
    ].join(" && ");

    Ok(Pod {
        metadata: ObjectMeta {
            name: Some(format!("task-{}", task.id)),
            namespace: Some(CONFIG.self_namespace().into()),
            ..Default::default()
        },
        spec: Some(PodSpec {
            service_account_name: Some(CONFIG.self_service_account_name().into()),
            containers: vec![Container {
                name: format!("task-{}", task.id),
                image: Some(CONFIG.helm_image().into()),
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
