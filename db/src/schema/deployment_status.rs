use chrono::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeploymentReportedStatus {
    timestamp: DateTime<Utc>,
    get_successful: bool,
    content: Option<DeploymentReportedStatusContent>,
    error: Option<String>,
}

impl DeploymentReportedStatus {
    pub fn new(content: DeploymentReportedStatusContent) -> Self {
        Self {
            timestamp: Utc::now(),
            get_successful: true,
            content: Some(content),
            error: None,
        }
    }

    pub fn new_error<E>(error: E) -> Self
    where
        E: std::fmt::Display,
    {
        Self {
            timestamp: Utc::now(),
            get_successful: false,
            content: None,
            error: Some(error.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeploymentReportedStatusContent {
    pub status: DeploymentReportedStatusSummary,
    pub primary_metric: Option<DeploymentRepootedMetric>,
    pub metrics: Option<Vec<DeploymentRepootedMetric>>,
    pub notices: Vec<DeploymentReportedNotice>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum DeploymentReportedStatusColor {
    Primary,
    Success,
    Danger,
    Warning,
    Secondary,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeploymentReportedStatusSummary {
    pub name: String,
    pub color: DeploymentReportedStatusColor,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeploymentRepootedMetric {
    pub value: Decimal,
    pub unit: String,
    pub short_description: String,
    pub color: Option<DeploymentReportedStatusColor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeploymentReportedNotice {
    pub level: DeploymentReportedNoticeLevel,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum DeploymentReportedNoticeLevel {
    Info,
    Warning,
    Danger,
}
