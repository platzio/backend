use anyhow::Result;
use log::*;
use platz_db::{Deployment, DeploymentTask, DeploymentTaskOperation, DeploymentTaskStatus, Json};

pub trait RunnableDeploymentTask: Send + Sync {
    async fn run(self) -> Result<()>;
}

impl RunnableDeploymentTask for DeploymentTask {
    #[tracing::instrument(ret, err, skip_all, name = "RDT.run")]
    async fn run(self) -> Result<()> {
        debug!("fetching deployment...");
        let deployment = Deployment::find(self.deployment_id).await?;
        debug!("updating status to Started...");
        self.set_status(DeploymentTaskStatus::Started, None).await?;
        debug!("status updated");

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

pub trait RunnableDeploymentOperation: Send + Sync {
    async fn run(&self, deployment: &Deployment, task: &DeploymentTask) -> Result<String>;
}
