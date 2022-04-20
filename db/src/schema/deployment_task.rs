use crate::json_diff::{json_diff, JsonDiff};
use crate::Deployment;
use crate::HelmChart;
use crate::K8sCluster;
use crate::NewDeploymentResourceType;
use crate::User;
use crate::{pool, DbError, DbResult, Paginated, DEFAULT_PAGE_SIZE};
use async_diesel::*;
use chrono::prelude::*;
use diesel::prelude::*;
use diesel::QueryDsl;
use diesel_derive_more::DBEnum;
use diesel_filter::{DieselFilter, Paginate};
pub use diesel_json::Json;
use platz_chart_ext::resource_types::ChartExtResourceType;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use uuid::Uuid;

table! {
    deployment_tasks(id) {
        id -> Uuid,
        created_at -> Timestamptz,
        first_attempted_at -> Nullable<Timestamptz>,
        started_at -> Nullable<Timestamptz>,
        finished_at -> Nullable<Timestamptz>,
        deployment_id -> Uuid,
        user_id -> Uuid,
        operation -> Jsonb,
        status -> Varchar,
        reason -> Nullable<Varchar>,
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
pub enum DeploymentTaskStatus {
    Pending,
    Started,
    Failed,
    Done,
}

impl Default for DeploymentTaskStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Debug, Identifiable, Queryable, Serialize, DieselFilter)]
#[table_name = "deployment_tasks"]
#[pagination]
pub struct DeploymentTask {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub first_attempted_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    #[filter]
    pub deployment_id: Uuid,
    pub user_id: Uuid,
    pub operation: Json<DeploymentTaskOperation>,
    pub status: DeploymentTaskStatus,
    pub reason: Option<String>,
}

impl DeploymentTask {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(deployment_tasks::table.get_results_async(pool()).await?)
    }

    pub async fn all_filtered(filters: DeploymentTaskFilters) -> DbResult<Paginated<Self>> {
        let conn = pool().get()?;
        let page = filters.page.unwrap_or(1);
        let per_page = filters.per_page.unwrap_or(DEFAULT_PAGE_SIZE);
        let (items, num_total) = tokio::task::spawn_blocking(move || {
            Self::filter(&filters)
                .paginate(Some(page))
                .per_page(Some(per_page))
                .load_and_count::<Self>(&conn)
        })
        .await
        .unwrap()?;
        Ok(Paginated {
            page,
            per_page,
            num_total,
            items,
        })
    }

    pub async fn find_by_deployment_id(deployment_id: Uuid) -> DbResult<Vec<Self>> {
        Ok(deployment_tasks::table
            .filter(deployment_tasks::deployment_id.eq(deployment_id))
            .get_results_async(pool())
            .await?)
    }

    pub async fn find(id: Uuid) -> DbResult<Self> {
        Ok(deployment_tasks::table
            .find(id)
            .get_result_async(pool())
            .await?)
    }

    pub async fn next_pending() -> DbResult<Option<Self>> {
        Ok(deployment_tasks::table
            .filter(deployment_tasks::status.eq(DeploymentTaskStatus::Pending))
            .order_by(deployment_tasks::created_at.asc())
            .get_result_async(pool())
            .await
            .optional()?)
    }

    pub async fn set_status(
        &self,
        status: DeploymentTaskStatus,
        reason: Option<String>,
    ) -> DbResult<Self> {
        let now = Utc::now();
        let (first_attempted_at, started_at, finished_at) = match (status, self.first_attempted_at)
        {
            (DeploymentTaskStatus::Pending, _) => (None, None, None),
            (DeploymentTaskStatus::Started, None) => (Some(now), Some(now), None),
            (DeploymentTaskStatus::Started, Some(_)) => (None, Some(now), None),
            (DeploymentTaskStatus::Failed, _) => (None, None, Some(now)),
            (DeploymentTaskStatus::Done, _) => (None, None, Some(now)),
        };
        UpdateDeploymentTask {
            first_attempted_at,
            started_at,
            finished_at,
            status: Some(status),
            reason: Some(reason),
        }
        .save(self.id)
        .await
    }

    pub async fn helm_chart(&self) -> DbResult<HelmChart> {
        let helm_chart_id = match &self.operation {
            Json(DeploymentTaskOperation::Install(params)) => params.helm_chart_id,
            Json(DeploymentTaskOperation::Upgrade(params)) => params.helm_chart_id,
            Json(DeploymentTaskOperation::InvokeAction(params)) => params.helm_chart_id,
            _ => {
                return Err(DbError::InvalidDeploymentRevision);
            }
        };
        HelmChart::find(helm_chart_id).await
    }

    pub async fn apply_deployment_resources(&self) -> DbResult<()> {
        let deployment = Deployment::find(self.deployment_id).await?;
        let cluster = K8sCluster::find(deployment.cluster_id).await?;
        let env_id = cluster
            .env_id
            .expect("Trying to apply deployment resources when a cluster has no env_id");
        let chart = self.helm_chart().await?;
        let types = chart.resource_types()?;
        for typ in types.inner.into_iter() {
            let ChartExtResourceType::V1Beta1(typ) = typ;
            NewDeploymentResourceType {
                env_id: if typ.spec.global { None } else { Some(env_id) },
                deployment_kind: deployment.kind.clone(),
                key: typ.key,
                spec: serde_json::to_value(typ.spec).unwrap(),
            }
            .save()
            .await?;
        }
        Ok(())
    }

    pub fn get_config(&self) -> DbResult<&serde_json::Value> {
        match &self.operation {
            Json(DeploymentTaskOperation::Install(params)) => Ok(&params.config_inputs),
            Json(DeploymentTaskOperation::Upgrade(params)) => Ok(&params.config_inputs),
            _ => Err(DbError::TaskHasNoConfig),
        }
    }

    pub async fn delete(&self) -> DbResult<()> {
        diesel::delete(deployment_tasks::table.find(self.id))
            .execute_async(pool())
            .await?;
        Ok(())
    }
}

#[derive(Insertable, Deserialize)]
#[table_name = "deployment_tasks"]
pub struct NewDeploymentTask {
    pub deployment_id: Uuid,
    pub user_id: Uuid,
    pub operation: Json<DeploymentTaskOperation>,
    pub status: DeploymentTaskStatus,
}

impl NewDeploymentTask {
    pub async fn insert(self) -> DbResult<DeploymentTask> {
        Ok(diesel::insert_into(deployment_tasks::table)
            .values(self)
            .get_result_async(pool())
            .await?)
    }
}

#[derive(AsChangeset)]
#[table_name = "deployment_tasks"]
pub struct UpdateDeploymentTask {
    pub first_attempted_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub status: Option<DeploymentTaskStatus>,
    pub reason: Option<Option<String>>,
}

impl UpdateDeploymentTask {
    pub async fn save(self, id: Uuid) -> DbResult<DeploymentTask> {
        Ok(
            diesel::update(deployment_tasks::table.filter(deployment_tasks::id.eq(id)))
                .set(self)
                .get_result_async(pool())
                .await?,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentTaskOperation {
    Install(DeploymentInstallTask),
    Upgrade(DeploymentUpgradeTask),
    Reinstall(DeploymentReinstallTask),
    Recreate(DeploymentRecreaseTask),
    Uninstall(DeploymentUninstallTask),
    InvokeAction(DeploymentInvokeActionTask),
    RestartK8sResource(DeploymentRestartK8sResourceTask),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentInstallTask {
    pub helm_chart_id: Uuid,
    pub config_inputs: serde_json::Value,
    pub values_override: Option<serde_json::Value>,
}

impl DeploymentTask {
    pub async fn create_install_task(deployment: &Deployment, user: &User) -> DbResult<Self> {
        NewDeploymentTask {
            deployment_id: deployment.id,
            user_id: user.id,
            operation: Json(DeploymentTaskOperation::Install(DeploymentInstallTask {
                helm_chart_id: deployment.helm_chart_id,
                config_inputs: deployment.config.clone(),
                values_override: deployment.values_override.clone(),
            })),
            status: Default::default(),
        }
        .insert()
        .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentUpgradeTask {
    pub helm_chart_id: Uuid,
    pub prev_helm_chart_id: Option<Uuid>,
    pub config_inputs: serde_json::Value,
    pub config_delta: Option<JsonDiff>,
    pub values_override: Option<serde_json::Value>,
}

impl DeploymentTask {
    pub async fn create_upgrade_task(
        old_deployment: &Deployment,
        new_deployment: &Deployment,
        user: &User,
    ) -> DbResult<Self> {
        NewDeploymentTask {
            deployment_id: new_deployment.id,
            user_id: user.id,
            operation: Json(DeploymentTaskOperation::Upgrade(DeploymentUpgradeTask {
                helm_chart_id: new_deployment.helm_chart_id,
                prev_helm_chart_id: Some(old_deployment.helm_chart_id),
                config_inputs: new_deployment.config.clone(),
                config_delta: Some(json_diff(&old_deployment.config, &new_deployment.config)),
                values_override: new_deployment.values_override.clone(),
            })),
            status: Default::default(),
        }
        .insert()
        .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentReinstallTask {
    pub reason: String,
}

impl DeploymentTask {
    pub async fn create_reinstall_task(
        deployment: &Deployment,
        user: &User,
        reason: String,
    ) -> DbResult<Self> {
        NewDeploymentTask {
            deployment_id: deployment.id,
            user_id: user.id,
            operation: Json(DeploymentTaskOperation::Reinstall(
                DeploymentReinstallTask {
                    reason: reason.clone(),
                },
            )),
            status: Default::default(),
        }
        .insert()
        .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentRecreaseTask {
    pub old_cluster_id: Uuid,
    pub old_namespace: String,
    pub new_cluster_id: Uuid,
    pub new_namespace: String,
}

impl DeploymentTask {
    pub async fn create_recreate_task(
        old_deployment: &Deployment,
        new_deployment: &Deployment,
        user: &User,
    ) -> DbResult<Self> {
        NewDeploymentTask {
            deployment_id: old_deployment.id,
            user_id: user.id,
            operation: Json(DeploymentTaskOperation::Recreate(DeploymentRecreaseTask {
                old_cluster_id: old_deployment.cluster_id,
                old_namespace: old_deployment.namespace_name(),
                new_cluster_id: new_deployment.cluster_id,
                new_namespace: new_deployment.namespace_name(),
            })),
            status: Default::default(),
        }
        .insert()
        .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentUninstallTask {}

impl DeploymentTask {
    pub async fn create_uninstall_task(deployment: &Deployment, user: &User) -> DbResult<Self> {
        NewDeploymentTask {
            deployment_id: deployment.id,
            user_id: user.id,
            operation: Json(DeploymentTaskOperation::Uninstall(
                DeploymentUninstallTask {},
            )),
            status: Default::default(),
        }
        .insert()
        .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentInvokeActionTask {
    pub helm_chart_id: Uuid,
    pub action_id: String,
    pub body: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentRestartK8sResourceTask {
    pub resource_id: Uuid,
    pub resource_name: String,
}
