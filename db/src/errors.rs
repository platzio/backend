use crate::events::DbEvent;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Diesel database error: {0}")]
    DieselError(#[from] async_diesel::AsyncError),

    #[error("Postgres database error: {0}")]
    PostgresError(#[from] postgres::Error),

    #[error("Failed parsing region name from helm registry domain name")]
    RegionNameParseError,

    #[error("Event parse error: {0}")]
    EventParseError(serde_json::Error),

    #[error("Event broadcast error: {0}")]
    EventBroadcastError(tokio::sync::broadcast::error::SendError<DbEvent>),

    #[error("Could not generate standard_ingress hostname because the cluster has no domain configured. Please configure a domain for the cluster and try again.")]
    ClusterHasNoDomain,

    #[error("This helm chart does not have any actions")]
    HelmChartNoActionsSchema,

    #[error("Helm chart features parsing error: {0}")]
    HelmChartFeaturesParsingError(serde_json::Error),

    #[error("Error parsing helm chart actions schema: {0}")]
    HelmChartActionsSchemaParseError(serde_json::Error),

    #[error("Error parsing helm chart values schema: {0}")]
    HelmChartValuesSchemaParseError(serde_json::Error),

    #[error("No such action ID: {0}")]
    HelmChartNoSuchAction(String),

    #[error("Invalid action body: {0}")]
    ActionBodyInvalid(String),

    #[error("Deployment has no revision_id")]
    DeploymentWithoutRevision,

    #[error("Deployment revision points to a task that doesn't have a helm_chart_id")]
    InvalidDeploymentRevision,

    #[error("Requested config_inputs from a task type that has no config inputs")]
    TaskHasNoConfig,
}

pub type DbResult<T> = Result<T, DbError>;
