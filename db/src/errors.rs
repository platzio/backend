#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Diesel database error: {0}")]
    DieselError(diesel::result::Error),

    #[error("Tokio join error: {0}")]
    TokioJoinError(#[from] tokio::task::JoinError),

    #[error("Database pool error: {0}")]
    Bb8Error(#[from] diesel_async::pooled_connection::bb8::RunError),

    #[error("Not found")]
    NotFound,

    #[error("Failed parsing region name from helm registry domain name")]
    RegionNameParseError,

    #[error(
        "Could not generate standard_ingress hostname because the cluster has no domain configured. Please configure a domain for the cluster and try again."
    )]
    ClusterHasNoIngressDomain,

    #[error("This helm chart does not have any actions")]
    HelmChartNoActionsSchema,

    #[error("Helm chart features parsing error: {0}")]
    HelmChartFeaturesParsingError(serde_json::Error),

    #[error("Error parsing helm chart actions schema: {0}")]
    HelmChartActionsSchemaParseError(serde_json::Error),

    #[error("Error parsing helm chart values schema: {0}")]
    HelmChartValuesSchemaParseError(serde_json::Error),

    #[error("Error parsing helm chart resource types: {0}")]
    HelmChartResourceTypesParseError(serde_json::Error),

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

    #[error("Error syncing deployment resource ({0}): {1}")]
    DeploymentResourceSyncError(String, String),
}

pub type DbResult<T> = Result<T, DbError>;

impl From<diesel::result::Error> for DbError {
    fn from(err: diesel::result::Error) -> Self {
        match err {
            diesel::result::Error::NotFound => Self::NotFound,
            err => Self::DieselError(err),
        }
    }
}
