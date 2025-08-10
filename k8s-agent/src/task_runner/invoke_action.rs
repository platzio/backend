use super::runnable_task::RunnableDeploymentOperation;
use crate::config::Config;
use anyhow::{anyhow, Result};
use platz_db::{
    schema::{
        deployment::Deployment,
        deployment_task::{DeploymentInvokeActionTask, DeploymentTask},
        k8s_cluster::K8sCluster,
    },
    DbError, DbTableOrDeploymentResource,
};
use tracing::debug;

impl RunnableDeploymentOperation for DeploymentInvokeActionTask {
    #[tracing::instrument(err, ret, name = "invoke_action", skip_all, fields(task_id = %task.id))]
    async fn run(
        &self,
        deployment: &Deployment,
        task: &DeploymentTask,
        _config: &Config,
    ) -> Result<String> {
        debug!("Loading chart...");
        let chart = task.helm_chart().await?;
        debug!("Loading cluster...");
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

        debug!("Requesting...");
        action_schema.target.call(deployment, body).await
    }
}
