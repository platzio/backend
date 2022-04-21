use anyhow::{bail, Result};
use platz_chart_ext::ChartExtActionEndpoint;
use platz_db::Deployment;
use std::time::Duration;
use tokio::time::{interval, Interval};
use url::Url;

#[derive(Clone, PartialEq, Eq)]
pub struct StatusConfig {
    pub url: Url,
    pub refresh_interval_secs: u64,
}

impl StatusConfig {
    pub async fn new(deployment: &Deployment) -> Result<Self> {
        let chart = deployment.current_helm_chart().await?;
        let features = chart.features()?;
        let status_feature = match features.status() {
            None => {
                bail!(
                    "Deployment {} ({}) doesn't have the status feature on, it won't be monitored",
                    deployment.id,
                    deployment.namespace_name()
                );
            }
            Some(status_feature) => status_feature,
        };

        let url = Url::parse(&match status_feature.endpoint {
            ChartExtActionEndpoint::StandardIngress => format!(
                "https://{}/{}",
                deployment.current_ingress_hostname().await?,
                status_feature.path.trim_start_matches('/')
            ),
        })?;

        let refresh_interval_secs = std::cmp::max(status_feature.refresh_interval_secs, 15);

        Ok(Self {
            url,
            refresh_interval_secs,
        })
    }

    pub fn interval(&self) -> Interval {
        interval(Duration::from_secs(self.refresh_interval_secs))
    }
}
