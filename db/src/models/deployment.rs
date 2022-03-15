use crate::pool;
use crate::DbTableOrDeploymentResource;
use crate::DeploymentTask;
use crate::HelmChart;
use crate::K8sCluster;
use crate::User;
use crate::{DbError, DbResult};
use async_diesel::*;
use chrono::prelude::*;
use diesel::prelude::*;
use diesel::QueryDsl;
use diesel_derive_more::DBEnum;
use platz_chart_ext::actions::{
    ChartExtActionEndpoint, ChartExtActionTarget, ChartExtActionTargetResolver,
};
use platz_chart_ext::UiSchema;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use url::Url;
use uuid::Uuid;

table! {
    deployments(id) {
        id -> Uuid,
        created_at -> Timestamptz,
        name -> Varchar,
        kind -> Varchar,
        cluster_id -> Uuid,
        enabled -> Bool,
        status -> Varchar,
        description_md -> Nullable<Varchar>,
        reason -> Nullable<Varchar>,
        revision_id -> Nullable<Uuid>,
        reported_status -> Nullable<Jsonb>,
        helm_chart_id -> Uuid,
        config -> Jsonb,
        values_override -> Nullable<Jsonb>,
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    EnumString,
    Display,
    AsExpression,
    FromSqlRow,
    DBEnum,
)]
#[sql_type = "diesel::sql_types::Text"]
pub enum DeploymentStatus {
    Unknown,
    Installing,
    Renaming,
    Upgrading,
    Running,
    Error,
    Uninstalling,
    Uninstalled,
    Deleting,
}

pub type DeploymentKind = String;

#[derive(Debug, Identifiable, Queryable, Serialize)]
pub struct Deployment {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub name: String,
    pub kind: DeploymentKind,
    pub cluster_id: Uuid,
    pub enabled: bool,
    pub status: DeploymentStatus,
    pub description_md: Option<String>,
    pub reason: Option<String>,
    pub revision_id: Option<Uuid>,
    pub reported_status: Option<serde_json::Value>,
    pub helm_chart_id: Uuid,
    pub config: serde_json::Value,
    pub values_override: Option<serde_json::Value>,
}

impl Deployment {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(deployments::table.get_results_async(pool()).await?)
    }

    pub async fn find(id: Uuid) -> DbResult<Self> {
        Ok(deployments::table.find(id).get_result_async(pool()).await?)
    }

    pub async fn find_optional(id: Uuid) -> DbResult<Option<Self>> {
        Ok(deployments::table
            .find(id)
            .get_result_async(pool())
            .await
            .optional()?)
    }

    pub async fn find_by_kind(kind: DeploymentKind) -> DbResult<Vec<Self>> {
        Ok(deployments::table
            .filter(deployments::kind.eq(kind))
            .get_results_async(pool())
            .await?)
    }

    async fn is_using(&self, collection: &DbTableOrDeploymentResource, id: &str) -> DbResult<bool> {
        let revision_id = match self.revision_id {
            Some(revision_id) => revision_id,
            None => return Ok(false),
        };
        let task = DeploymentTask::find(revision_id).await?;
        let chart = match task.helm_chart().await {
            Ok(chart) => chart,
            Err(DbError::InvalidDeploymentRevision) => return Ok(false),
            Err(err) => return Err(err),
        };
        let values_ui: UiSchema = match chart.values_ui {
            None => return Ok(false),
            Some(diesel_json::Json(values_ui)) => values_ui,
        };
        let config = match task.get_config() {
            Ok(config) => config,
            Err(DbError::TaskHasNoConfig) => return Ok(false),
            Err(err) => return Err(err),
        };
        Ok(values_ui.is_collection_in_inputs(config, collection, id))
    }

    pub async fn find_using(
        collection: &DbTableOrDeploymentResource,
        id: Uuid,
    ) -> DbResult<Vec<Self>> {
        let id = id.to_string();
        let mut result = Vec::new();
        for deployment in Self::all().await?.into_iter() {
            if deployment.is_using(collection, &id).await? {
                result.push(deployment);
            }
        }
        Ok(result)
    }

    pub async fn reinstall_all_using(
        collection: &DbTableOrDeploymentResource,
        id: Uuid,
        user: &User,
        reason: String,
    ) -> DbResult<()> {
        for deployment in Deployment::find_using(collection, id)
            .await?
            .into_iter()
            .filter(|deployment| deployment.enabled)
        {
            DeploymentTask::create_reinstall_task(&deployment, user, reason.clone()).await?;
        }
        Ok(())
    }

    pub async fn find_by_cluster_id(cluster_id: Uuid) -> DbResult<Vec<Self>> {
        Ok(deployments::table
            .filter(deployments::cluster_id.eq(cluster_id))
            .get_results_async(pool())
            .await?)
    }

    pub async fn find_by_env_id(env_id: Uuid) -> DbResult<Vec<Self>> {
        let mut result = Vec::new();
        for cluster in K8sCluster::find_by_env_id(env_id).await? {
            let mut deployments = deployments::table
                .filter(deployments::cluster_id.eq(cluster.id))
                .get_results_async(pool())
                .await?;
            result.append(&mut deployments);
        }
        Ok(result)
    }

    pub async fn all_with_ongoing_clearing_status_in_cluster(
        cluster_id: Uuid,
    ) -> DbResult<Vec<Self>> {
        Ok(deployments::table
            .filter(
                deployments::status
                    .eq(DeploymentStatus::Deleting)
                    .or(deployments::status.eq(DeploymentStatus::Uninstalling)),
            )
            .filter(deployments::cluster_id.eq(cluster_id))
            .get_results_async(pool())
            .await?)
    }

    pub async fn reinstall_all_for_env(env_id: Uuid, user: &User, reason: String) -> DbResult<()> {
        for deployment in Self::find_by_env_id(env_id)
            .await?
            .into_iter()
            .filter(|deployment| deployment.enabled)
        {
            DeploymentTask::create_reinstall_task(&deployment, user, reason.clone()).await?;
        }
        Ok(())
    }

    pub async fn find_by_cluster_and_kind(
        cluster_id: Uuid,
        kind: DeploymentKind,
    ) -> DbResult<Vec<Self>> {
        Ok(deployments::table
            .filter(deployments::cluster_id.eq(cluster_id))
            .filter(deployments::kind.eq(kind))
            .get_results_async(pool())
            .await?)
    }

    pub fn namespace_name(&self) -> String {
        let kind = self.kind.to_string().to_lowercase();
        if self.name.is_empty() {
            kind
        } else {
            format!("{}-{}", kind, self.name)
        }
    }

    pub async fn standard_ingress_hostname(&self) -> DbResult<String> {
        let cluster = K8sCluster::find(self.cluster_id).await?;
        Ok(format!(
            "{}.{}",
            self.namespace_name(),
            cluster.domain.ok_or(DbError::ClusterHasNoDomain)?
        ))
    }

    pub async fn revision_task(&self) -> DbResult<DeploymentTask> {
        DeploymentTask::find(self.revision_id.ok_or(DbError::DeploymentWithoutRevision)?).await
    }

    pub async fn current_helm_chart(&self) -> DbResult<HelmChart> {
        self.revision_task().await?.helm_chart().await
    }

    pub async fn set_status(
        &self,
        status: DeploymentStatus,
        reason: Option<String>,
    ) -> DbResult<Self> {
        Ok(UpdateDeploymentStatus {
            status: Some(status),
            reason: Some(reason),
            revision_id: None,
        }
        .save(self.id)
        .await?)
    }

    pub async fn set_status_and_revision(
        &self,
        status: DeploymentStatus,
        reason: Option<String>,
        revision_id: Uuid,
    ) -> DbResult<Self> {
        Ok(UpdateDeploymentStatus {
            status: Some(status),
            reason: Some(reason),
            revision_id: Some(Some(revision_id)),
        }
        .save(self.id)
        .await?)
    }

    pub async fn set_revision(&self, revision_id: Option<Uuid>) -> DbResult<Self> {
        Ok(UpdateDeploymentStatus {
            status: None,
            reason: None,
            revision_id: Some(revision_id),
        }
        .save(self.id)
        .await?)
    }

    pub async fn delete(&self) -> DbResult<()> {
        diesel::delete(deployments::table.find(self.id))
            .execute_async(pool())
            .await?;
        Ok(())
    }
}

#[derive(Insertable, Deserialize)]
#[table_name = "deployments"]
pub struct NewDeployment {
    #[serde(default)]
    pub name: String,
    pub kind: DeploymentKind,
    pub cluster_id: Uuid,
    pub helm_chart_id: Uuid,
    pub config: Option<serde_json::Value>,
    pub values_override: Option<serde_json::Value>,
}

impl NewDeployment {
    pub async fn insert(self) -> DbResult<Deployment> {
        Ok(diesel::insert_into(deployments::table)
            .values(self)
            .get_result_async(pool())
            .await?)
    }
}

#[derive(AsChangeset, Deserialize)]
#[table_name = "deployments"]
pub struct UpdateDeployment {
    pub name: Option<String>,
    pub cluster_id: Option<Uuid>,
    pub helm_chart_id: Option<Uuid>,
    pub config: Option<serde_json::Value>,
    pub values_override: Option<Option<serde_json::Value>>,
    pub enabled: Option<bool>,
    pub description_md: Option<String>,
}

impl UpdateDeployment {
    pub async fn save(self, id: Uuid) -> DbResult<Deployment> {
        Ok(
            diesel::update(deployments::table.filter(deployments::id.eq(id)))
                .set(self)
                .get_result_async(pool())
                .await?,
        )
    }
}

#[derive(AsChangeset)]
#[table_name = "deployments"]
pub struct UpdateDeploymentStatus {
    pub status: Option<DeploymentStatus>,
    pub reason: Option<Option<String>>,
    pub revision_id: Option<Option<Uuid>>,
}

impl UpdateDeploymentStatus {
    pub async fn save(self, id: Uuid) -> DbResult<Deployment> {
        Ok(
            diesel::update(deployments::table.filter(deployments::id.eq(id)))
                .set(self)
                .get_result_async(pool())
                .await?,
        )
    }
}

#[derive(AsChangeset)]
#[table_name = "deployments"]
pub struct UpdateDeploymentReportedStatus {
    pub reported_status: Option<Option<serde_json::Value>>,
}

impl UpdateDeploymentReportedStatus {
    pub async fn save(self, id: Uuid) -> DbResult<Deployment> {
        Ok(
            diesel::update(deployments::table.filter(deployments::id.eq(id)))
                .set(self)
                .get_result_async(pool())
                .await?,
        )
    }
}

#[async_trait::async_trait]
impl ChartExtActionTargetResolver for Deployment {
    async fn resolve(&self, target: &ChartExtActionTarget) -> anyhow::Result<Url> {
        let host = match target.endpoint {
            ChartExtActionEndpoint::StandardIngress => self.standard_ingress_hostname().await?,
        };
        Ok(Url::parse(&format!(
            "https://{}/{}",
            host,
            target.path.trim_start_matches('/'),
        ))?)
    }
}
