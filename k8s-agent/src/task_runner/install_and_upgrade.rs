use super::helm::run_helm;
use super::runnable_task::RunnableDeploymentOperation;
use crate::deployment_creds::apply_deployment_credentials;
use crate::k8s::K8S_TRACKER;
use crate::k8s::{deployment_namespace_annotations, DEPLOYMENT_NAMESPACE_LABELS};
use anyhow::Result;
use async_trait::async_trait;
use k8s_openapi::api::core::v1::Namespace;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::api::Api;
use log::debug;
use platz_db::{
    Deployment, DeploymentInstallTask, DeploymentRecreaseTask, DeploymentReinstallTask,
    DeploymentStatus, DeploymentTask, DeploymentUninstallTask, DeploymentUpgradeTask,
};
use uuid::Uuid;

#[async_trait]
impl RunnableDeploymentOperation for DeploymentInstallTask {
    async fn run(&self, deployment: &Deployment, task: &DeploymentTask) -> Result<String> {
        deployment
            .set_status(DeploymentStatus::Installing, None)
            .await?;
        create_namespace(deployment.cluster_id, deployment_to_namespace(deployment)).await?;
        apply_deployment_credentials(deployment).await?;
        match run_helm("install", deployment, task).await {
            Ok(output) => {
                deployment.set_revision(Some(task.id)).await?;
                task.apply_deployment_resources().await?;
                deployment
                    .set_status(DeploymentStatus::Running, None)
                    .await?;
                Ok(output)
            }
            Err(err) => {
                deployment
                    .set_status(DeploymentStatus::Error, Some(err.to_string()))
                    .await?;
                Err(err)
            }
        }
    }
}

#[async_trait]
impl RunnableDeploymentOperation for DeploymentUpgradeTask {
    async fn run(&self, deployment: &Deployment, task: &DeploymentTask) -> Result<String> {
        deployment
            .set_status(DeploymentStatus::Upgrading, None)
            .await?;
        match run_helm("upgrade --install", deployment, task).await {
            Ok(output) => {
                deployment.set_revision(Some(task.id)).await?;
                task.apply_deployment_resources().await?;
                deployment
                    .set_status(DeploymentStatus::Running, None)
                    .await?;
                Ok(output)
            }
            Err(err) => {
                deployment
                    .set_status(DeploymentStatus::Error, Some(err.to_string()))
                    .await?;
                Err(err)
            }
        }
    }
}

#[async_trait]
impl RunnableDeploymentOperation for DeploymentReinstallTask {
    async fn run(&self, deployment: &Deployment, _task: &DeploymentTask) -> Result<String> {
        deployment
            .set_status(DeploymentStatus::Upgrading, None)
            .await?;
        let revision_task = deployment.revision_task().await?;
        match run_helm("upgrade --install", deployment, &revision_task).await {
            Ok(output) => {
                revision_task.apply_deployment_resources().await?;
                deployment
                    .set_status(DeploymentStatus::Running, None)
                    .await?;
                Ok(output)
            }
            Err(err) => {
                deployment
                    .set_status(DeploymentStatus::Error, Some(err.to_string()))
                    .await?;
                Err(err)
            }
        }
    }
}

#[async_trait]
impl RunnableDeploymentOperation for DeploymentRecreaseTask {
    async fn run(&self, deployment: &Deployment, _task: &DeploymentTask) -> Result<String> {
        deployment
            .set_status(DeploymentStatus::Renaming, None)
            .await?;
        delete_namespace(self.old_cluster_id, &self.old_namespace).await?;
        create_namespace(self.new_cluster_id, deployment_to_namespace(deployment)).await?;
        apply_deployment_credentials(deployment).await?;
        Ok("".to_owned())
    }
}

#[async_trait]
impl RunnableDeploymentOperation for DeploymentUninstallTask {
    async fn run(&self, deployment: &Deployment, _task: &DeploymentTask) -> Result<String> {
        if deployment.status == DeploymentStatus::Deleting {
            deployment
                .set_status(DeploymentStatus::Deleting, None)
                .await?;
        } else {
            deployment
                .set_status(DeploymentStatus::Uninstalling, None)
                .await?;
        }
        delete_namespace(deployment.cluster_id, &deployment.namespace_name()).await?;
        deployment.set_revision(None).await?;
        Ok("".to_owned())
    }
}

// --------------------
// Namespace operations
// --------------------

fn deployment_to_namespace(deployment: &Deployment) -> Namespace {
    Namespace {
        metadata: ObjectMeta {
            name: Some(deployment.namespace_name()),
            labels: Some(DEPLOYMENT_NAMESPACE_LABELS.to_owned()),
            annotations: Some(deployment_namespace_annotations(deployment)),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[tracing::instrument(err, skip(namespace), fields(namespace=namespace.metadata.name))]
async fn create_namespace(cluster_id: Uuid, namespace: Namespace) -> Result<()> {
    debug!("creating");
    let api = Api::all(
        K8S_TRACKER
            .get_cluster(cluster_id)
            .await?
            .kube_client()
            .await?,
    );
    api.create(&Default::default(), &namespace).await?;
    debug!("created");
    Ok(())
}

#[tracing::instrument(err)]
async fn delete_namespace(cluster_id: Uuid, namespace_name: &str) -> Result<()> {
    debug!("Deleting");
    let api = Api::<Namespace>::all(
        K8S_TRACKER
            .get_cluster(cluster_id)
            .await?
            .kube_client()
            .await?,
    );
    if let Err(e) = api.delete(namespace_name, &Default::default()).await {
        tracing::error!(?e);
        if let kube::Error::Api(kube::core::ErrorResponse { code, .. }) = e {
            if http::StatusCode::NOT_FOUND == code {
                // If it's not found, I guess it is... *drums roll* DELETED *cymbals*
                debug!("Namespace not found - ignoring");
                return Ok(());
            }
        }
        return Err(e.into());
    }
    Ok(())
}
