use super::RunnableDeploymentOperation;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use platz_db::{
    DbError, DbTableOrDeploymentResource, Deployment, DeploymentInvokeActionTask, DeploymentTask,
    K8sCluster,
};

#[async_trait]
impl RunnableDeploymentOperation for DeploymentInvokeActionTask {
    async fn run(&self, deployment: &Deployment, task: &DeploymentTask) -> Result<String> {
        let chart = task.helm_chart().await?;
        let cluster = K8sCluster::find(deployment.cluster_id).await?;
        let env_id = cluster.env_id.ok_or_else(|| {
            anyhow!("Can't invoke action for a deployment in a cluster with no env_id")
        })?;
        let actions_schema = chart.actions_schema()?;
        let action_schema = actions_schema
            .find(&self.action_id)
            .ok_or_else(|| DbError::HelmChartNoSuchAction(self.action_id.to_owned()))?;

        let body = action_schema
            .generate_body::<DbTableOrDeploymentResource>(env_id, self.body.clone())
            .await?;

        action_schema.target.call(deployment, body).await
    }
}
