mod helm;
mod install_and_upgrade;
mod invoke_action;
mod restart_k8s_resource;
mod secrets;
mod values;

use anyhow::Result;
use async_trait::async_trait;
use log::*;
use platz_db::{Deployment, DeploymentTask, DeploymentTaskOperation, DeploymentTaskStatus, Json};

#[async_trait]
pub trait RunnableDeploymentTask: Send + Sync {
    async fn run(self) -> Result<()>;
}

#[async_trait]
impl RunnableDeploymentTask for DeploymentTask {
    async fn run(self) -> Result<()> {
        debug!("Running DeploymentTask {}", self.id);
        let deployment = Deployment::find(self.deployment_id).await?;

        self.set_status(DeploymentTaskStatus::Started, None).await?;

        let result = match &self.operation {
            Json(DeploymentTaskOperation::Install(inner)) => inner.run(&deployment, &self).await,
            Json(DeploymentTaskOperation::Upgrade(inner)) => inner.run(&deployment, &self).await,
            Json(DeploymentTaskOperation::Recreate(inner)) => inner.run(&deployment, &self).await,
            Json(DeploymentTaskOperation::Reinstall(inner)) => inner.run(&deployment, &self).await,
            Json(DeploymentTaskOperation::Uninstall(inner)) => inner.run(&deployment, &self).await,
            Json(DeploymentTaskOperation::InvokeAction(inner)) => {
                inner.run(&deployment, &self).await
            }
            Json(DeploymentTaskOperation::RestartK8sResource(inner)) => {
                inner.run(&deployment, &self).await
            }
        };

        match result {
            Ok(reason) => {
                self.set_status(DeploymentTaskStatus::Done, Some(reason))
                    .await?;
                Ok(())
            }
            Err(err) => {
                self.set_status(DeploymentTaskStatus::Failed, Some(err.to_string()))
                    .await?;
                Err(err)
            }
        }
    }
}

#[async_trait]
pub trait RunnableDeploymentOperation: Send + Sync {
    async fn run(&self, deployment: &Deployment, task: &DeploymentTask) -> Result<String>;
}
