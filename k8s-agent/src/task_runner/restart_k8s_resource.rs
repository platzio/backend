use super::runnable_task::RunnableDeploymentOperation;
use crate::k8s::K8S_TRACKER;
use anyhow::anyhow;
use anyhow::Result;
use kube::api::Api;
use platz_db::{Deployment, DeploymentRestartK8sResourceTask, DeploymentTask, K8sResource};

impl RunnableDeploymentOperation for DeploymentRestartK8sResourceTask {
    async fn run(&self, deployment: &Deployment, _task: &DeploymentTask) -> Result<String> {
        let resource = K8sResource::find(self.resource_id).await?.ok_or_else(|| {
            anyhow!(
                "Unknown resource {} ({})",
                self.resource_name,
                self.resource_id
            )
        })?;

        let client = K8S_TRACKER
            .get_cluster(deployment.cluster_id)
            .await?
            .kube_client()
            .await?;
        let ns = deployment.namespace_name().await;

        match resource.kind.as_str() {
            "Deployment" => {
                let api = Api::<k8s_openapi::api::apps::v1::Deployment>::namespaced(client, &ns);
                api.restart(&resource.name).await?;
                Ok("".to_owned())
            }
            "StatefulSet" => {
                let api = Api::<k8s_openapi::api::apps::v1::StatefulSet>::namespaced(client, &ns);
                api.restart(&resource.name).await?;
                Ok("".to_owned())
            }
            _ => Err(anyhow!(
                "Resource {} of kind {} doesn't support restart",
                resource.name,
                resource.kind
            )),
        }
    }
}
