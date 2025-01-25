use crate::json_diff::{json_diff, JsonDiff};
use crate::Deployment;
use crate::HelmChart;
use crate::Identity;
use crate::K8sCluster;
use crate::NewDeploymentResourceType;
use crate::{db_conn, DbError, DbResult, Paginated, DEFAULT_PAGE_SIZE};
use chrono::prelude::*;
use diesel::prelude::*;
use diesel::QueryDsl;
use diesel_async::RunQueryDsl;
use diesel_enum_derive::DieselEnum;
use diesel_filter::{DieselFilter, Paginate};
use diesel_json::Json;
use platz_chart_ext::resource_types::ChartExtResourceType;
use serde::{Deserialize, Serialize};
use std::ops::DerefMut;
use strum::{AsRefStr, Display, EnumIter, EnumString};
use utoipa::ToSchema;
use uuid::Uuid;

table! {
    deployment_tasks(id) {
        id -> Uuid,
        created_at -> Timestamptz,
        execute_at -> Timestamptz,
        first_attempted_at -> Nullable<Timestamptz>,
        started_at -> Nullable<Timestamptz>,
        finished_at -> Nullable<Timestamptz>,
        cluster_id -> Uuid,
        deployment_id -> Uuid,
        acting_user_id -> Nullable<Uuid>,
        acting_deployment_id -> Nullable<Uuid>,
        canceled_by_user_id -> Nullable<Uuid>,
        canceled_by_deployment_id -> Nullable<Uuid>,
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
    EnumIter,
    AsRefStr,
    Display,
    DieselEnum,
    ToSchema,
)]
pub enum DeploymentTaskStatus {
    Pending,
    Started,
    Failed,
    Canceled,
    Done,
}

impl Default for DeploymentTaskStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Debug, Identifiable, Queryable, Serialize, DieselFilter, ToSchema)]
#[diesel(table_name = deployment_tasks)]
#[pagination]
pub struct DeploymentTask {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub execute_at: DateTime<Utc>,
    #[schema(required)]
    pub first_attempted_at: Option<DateTime<Utc>>,
    #[schema(required)]
    pub started_at: Option<DateTime<Utc>>,
    #[schema(required)]
    pub finished_at: Option<DateTime<Utc>>,
    #[filter]
    pub cluster_id: Uuid,
    #[filter]
    pub deployment_id: Uuid,
    #[schema(required)]
    pub acting_user_id: Option<Uuid>,
    #[schema(required)]
    pub acting_deployment_id: Option<Uuid>,
    #[schema(required)]
    pub canceled_by_user_id: Option<Uuid>,
    #[schema(required)]
    pub canceled_by_deployment_id: Option<Uuid>,
    #[schema(value_type = DeploymentTaskOperation)]
    pub operation: Json<DeploymentTaskOperation>,
    pub status: DeploymentTaskStatus,
    #[schema(required)]
    pub reason: Option<String>,
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct DeploymentTaskExtraFilters {
    #[schema(required)]
    active_only: Option<bool>,
    #[schema(required)]
    show_future: Option<bool>,
    #[schema(required)]
    created_from: Option<DateTime<Utc>>,
    #[schema(required)]
    env_id: Option<Uuid>,
}

#[derive(Queryable)]
pub struct DeploymentTaskStat {
    pub count: i64,
    pub status: DeploymentTaskStatus,
}

impl DeploymentTask {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(deployment_tasks::table
            .get_results(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn all_filtered(
        filters: DeploymentTaskFilters,
        extra_filters: DeploymentTaskExtraFilters,
    ) -> DbResult<Paginated<Self>> {
        let page = filters.page.unwrap_or(1);
        let per_page = filters.per_page.unwrap_or(DEFAULT_PAGE_SIZE);
        let allowed_cluster_ids: Option<Vec<Uuid>> = if let Some(env_id) = extra_filters.env_id {
            Some(
                K8sCluster::find_by_env_id(env_id)
                    .await?
                    .iter()
                    .map(|cluster| cluster.id)
                    .collect(),
            )
        } else {
            None
        };
        let mut filtered = Self::filter(filters);
        if extra_filters.active_only.unwrap_or(false) {
            filtered = filtered.filter(
                deployment_tasks::status
                    .eq(DeploymentTaskStatus::Started)
                    .or(deployment_tasks::status.eq(DeploymentTaskStatus::Pending)),
            );
        }
        if !extra_filters.show_future.unwrap_or(true) {
            filtered = filtered.filter(deployment_tasks::execute_at.le(diesel::dsl::now));
        }
        if let Some(from_date_time) = extra_filters.created_from {
            filtered = filtered.filter(deployment_tasks::created_at.ge(from_date_time))
        }
        if let Some(cluster_ids) = allowed_cluster_ids {
            filtered = filtered.filter(deployment_tasks::cluster_id.eq_any(cluster_ids))
        }
        let (items, num_total) = filtered
            .order_by(deployment_tasks::execute_at.desc())
            .paginate(Some(page))
            .per_page(Some(per_page))
            .load_and_count(db_conn().await?.deref_mut())
            .await?;
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
            .get_results(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn find(id: Uuid) -> DbResult<Self> {
        Ok(deployment_tasks::table
            .find(id)
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn next_pending(cluster_ids: &Vec<Uuid>) -> DbResult<Option<Self>> {
        Ok(deployment_tasks::table
            .filter(deployment_tasks::status.eq(DeploymentTaskStatus::Pending))
            .filter(deployment_tasks::cluster_id.eq_any(cluster_ids.to_owned()))
            .filter(deployment_tasks::execute_at.le(diesel::dsl::now))
            .order_by(deployment_tasks::execute_at.asc())
            .get_result(db_conn().await?.deref_mut())
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
            (DeploymentTaskStatus::Canceled, _) => (None, None, None),
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
        for typ in types.0.into_iter() {
            let ChartExtResourceType::V1Beta1(typ) = typ;
            NewDeploymentResourceType {
                env_id: if typ.spec.global { None } else { Some(env_id) },
                deployment_kind_id: deployment.kind_id,
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
            .execute(db_conn().await?.deref_mut())
            .await?;
        Ok(())
    }

    pub async fn get_status_counters() -> DbResult<Vec<DeploymentTaskStat>> {
        Ok(deployment_tasks::table
            .group_by(deployment_tasks::status)
            .select((diesel::dsl::count_star(), deployment_tasks::status))
            .get_results(db_conn().await?.deref_mut())
            .await?)
    }
}

#[derive(Insertable, Deserialize, ToSchema)]
#[diesel(table_name = deployment_tasks)]
pub struct NewDeploymentTask {
    pub cluster_id: Uuid,
    pub deployment_id: Uuid,
    #[schema(required)]
    pub acting_user_id: Option<Uuid>,
    #[schema(required)]
    pub acting_deployment_id: Option<Uuid>,
    #[schema(value_type = DeploymentTaskOperation)]
    pub operation: Json<DeploymentTaskOperation>,
    pub status: DeploymentTaskStatus,
    #[schema(required)]
    pub execute_at: Option<DateTime<Utc>>,
}

impl NewDeploymentTask {
    pub async fn insert(self) -> DbResult<DeploymentTask> {
        Ok(diesel::insert_into(deployment_tasks::table)
            .values(self)
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }
}

#[derive(AsChangeset)]
#[diesel(table_name = deployment_tasks)]
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
                .get_result(db_conn().await?.deref_mut())
                .await?,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum DeploymentTaskOperation {
    Install(DeploymentInstallTask),
    Upgrade(DeploymentUpgradeTask),
    Reinstall(DeploymentReinstallTask),
    Recreate(DeploymentRecreaseTask),
    Uninstall(DeploymentUninstallTask),
    InvokeAction(DeploymentInvokeActionTask),
    RestartK8sResource(DeploymentRestartK8sResourceTask),
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeploymentInstallTask {
    pub helm_chart_id: Uuid,
    pub config_inputs: serde_json::Value,
    #[schema(required)]
    pub values_override: Option<serde_json::Value>,
}

impl DeploymentTask {
    pub async fn create_install_task<I>(deployment: &Deployment, identity: &I) -> DbResult<Self>
    where
        I: std::borrow::Borrow<Identity>,
    {
        NewDeploymentTask {
            cluster_id: deployment.cluster_id,
            deployment_id: deployment.id,
            acting_user_id: identity.borrow().user_id(),
            acting_deployment_id: identity.borrow().deployment_id(),
            operation: Json(DeploymentTaskOperation::Install(DeploymentInstallTask {
                helm_chart_id: deployment.helm_chart_id,
                config_inputs: deployment.config.clone(),
                values_override: deployment.values_override.clone(),
            })),
            status: Default::default(),
            execute_at: None,
        }
        .insert()
        .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeploymentUpgradeTask {
    pub helm_chart_id: Uuid,
    #[schema(required)]
    pub prev_helm_chart_id: Option<Uuid>,
    pub config_inputs: serde_json::Value,
    #[schema(required)]
    pub config_delta: Option<JsonDiff>,
    #[schema(required)]
    pub values_override: Option<serde_json::Value>,
}

impl DeploymentTask {
    pub async fn create_upgrade_task<I>(
        old_deployment: &Deployment,
        new_deployment: &Deployment,
        identity: &I,
    ) -> DbResult<Self>
    where
        I: std::borrow::Borrow<Identity>,
    {
        NewDeploymentTask {
            cluster_id: new_deployment.cluster_id,
            deployment_id: new_deployment.id,
            acting_user_id: identity.borrow().user_id(),
            acting_deployment_id: identity.borrow().deployment_id(),
            operation: Json(DeploymentTaskOperation::Upgrade(DeploymentUpgradeTask {
                helm_chart_id: new_deployment.helm_chart_id,
                prev_helm_chart_id: Some(old_deployment.helm_chart_id),
                config_inputs: new_deployment.config.clone(),
                config_delta: Some(json_diff(&old_deployment.config, &new_deployment.config)),
                values_override: new_deployment.values_override.clone(),
            })),
            status: Default::default(),
            execute_at: None,
        }
        .insert()
        .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeploymentReinstallTask {
    pub reason: String,
}

impl DeploymentTask {
    pub async fn create_reinstall_task<I>(
        deployment: &Deployment,
        identity: &I,
        reason: String,
    ) -> DbResult<Self>
    where
        I: std::borrow::Borrow<Identity>,
    {
        NewDeploymentTask {
            cluster_id: deployment.cluster_id,
            deployment_id: deployment.id,
            acting_user_id: identity.borrow().user_id(),
            acting_deployment_id: identity.borrow().deployment_id(),
            operation: Json(DeploymentTaskOperation::Reinstall(
                DeploymentReinstallTask {
                    reason: reason.clone(),
                },
            )),
            status: Default::default(),
            execute_at: None,
        }
        .insert()
        .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeploymentRecreaseTask {
    pub old_cluster_id: Uuid,
    pub old_namespace: String,
    pub new_cluster_id: Uuid,
    pub new_namespace: String,
}

impl DeploymentTask {
    pub async fn create_recreate_task<I>(
        old_deployment: &Deployment,
        new_deployment: &Deployment,
        identity: &I,
    ) -> DbResult<Self>
    where
        I: std::borrow::Borrow<Identity>,
    {
        NewDeploymentTask {
            // TODO: See https://github.com/platzio/backend/issues/20
            cluster_id: new_deployment.cluster_id,
            deployment_id: new_deployment.id,
            acting_user_id: identity.borrow().user_id(),
            acting_deployment_id: identity.borrow().deployment_id(),
            operation: Json(DeploymentTaskOperation::Recreate(DeploymentRecreaseTask {
                old_cluster_id: old_deployment.cluster_id,
                old_namespace: old_deployment.namespace_name().await,
                new_cluster_id: new_deployment.cluster_id,
                new_namespace: new_deployment.namespace_name().await,
            })),
            status: Default::default(),
            execute_at: None,
        }
        .insert()
        .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeploymentUninstallTask {}

impl DeploymentTask {
    pub async fn create_uninstall_task<I>(deployment: &Deployment, identity: &I) -> DbResult<Self>
    where
        I: std::borrow::Borrow<Identity>,
    {
        NewDeploymentTask {
            cluster_id: deployment.cluster_id,
            deployment_id: deployment.id,
            acting_user_id: identity.borrow().user_id(),
            acting_deployment_id: identity.borrow().deployment_id(),
            operation: Json(DeploymentTaskOperation::Uninstall(
                DeploymentUninstallTask {},
            )),
            status: Default::default(),
            execute_at: None,
        }
        .insert()
        .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeploymentInvokeActionTask {
    pub helm_chart_id: Uuid,
    pub action_id: String,
    pub body: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeploymentRestartK8sResourceTask {
    pub resource_id: Uuid,
    pub resource_name: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = deployment_tasks)]
pub struct CancelDeploymentTask {
    pub canceled_by_user_id: Option<Uuid>,
    pub canceled_by_deployment_id: Option<Uuid>,
    pub reason: Option<String>,
}

impl CancelDeploymentTask {
    pub async fn save(self, id: Uuid) -> DbResult<DeploymentTask> {
        Ok(
            diesel::update(deployment_tasks::table.filter(deployment_tasks::id.eq(id)))
                .set((
                    self,
                    deployment_tasks::status.eq(DeploymentTaskStatus::Canceled),
                    deployment_tasks::finished_at.eq(diesel::dsl::now),
                ))
                .get_result(db_conn().await?.deref_mut())
                .await?,
        )
    }
}
