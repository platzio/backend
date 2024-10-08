use chrono::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use strum::Display;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeploymentReportedStatus {
    timestamp: DateTime<Utc>,
    get_successful: bool,
    #[schema(required)]
    content: Option<DeploymentReportedStatusContent>,
    #[schema(required)]
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
    #[schema(required)]
    pub primary_metric: Option<DeploymentReportedMetric>,
    #[schema(required)]
    pub metrics: Option<Vec<DeploymentReportedMetric>>,
    #[serde(default)]
    pub notices: Vec<DeploymentReportedNotice>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Display, ToSchema)]
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
pub struct DeploymentReportedMetric {
    pub value: Decimal,
    pub unit: String,
    pub short_description: String,
    #[schema(required)]
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
