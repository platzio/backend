use super::RunnableDeploymentOperation;
use anyhow::Result;
use async_trait::async_trait;
use platz_db::{Deployment, DeploymentInvokeActionTask, DeploymentTask, HelmChartActionEndpoint};
use std::str::FromStr;
use url::Url;

#[async_trait]
impl RunnableDeploymentOperation for DeploymentInvokeActionTask {
    async fn run(&self, deployment: &Deployment, task: &DeploymentTask) -> Result<String> {
        let chart = task.helm_chart().await?;
        let actions_schema = chart.actions_schema()?;
        let action_schema = actions_schema.find(&self.action_id)?;

        let url = Url::parse(&format!(
            "https://{}/{}",
            match action_schema.endpoint {
                HelmChartActionEndpoint::StandardIngress =>
                    deployment.standard_ingress_hostname().await?,
            },
            action_schema.path.trim_start_matches('/'),
        ))?;
        let method = reqwest::Method::from_str(&action_schema.method.to_uppercase())?;
        let body = action_schema.generate_body(self.body.clone()).await?;

        Ok(reqwest::Client::new()
            .request(method, url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?)
    }
}
