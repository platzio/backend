use crate::Deployment;
use crate::DeploymentResourceType;
use crate::{pool, DbError, DbResult, Paginated, DEFAULT_PAGE_SIZE};
use async_diesel::*;
use chrono::prelude::*;
use diesel::prelude::*;
use diesel_enum_derive::DieselEnum;
use diesel_filter::{DieselFilter, Paginate};
use platz_chart_ext::ChartExtActionTarget;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use tracing::warn;
use utoipa::ToSchema;
use uuid::Uuid;

table! {
    deployment_resources(id) {
        id -> Uuid,
        created_at -> Timestamptz,
        type_id -> Uuid,
        deployment_id -> Nullable<Uuid>,
        name -> Varchar,
        exists -> Bool,
        props -> Jsonb,
        sync_status -> Varchar,
        sync_reason -> Nullable<Varchar>,
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
    DieselEnum,
    ToSchema,
)]
pub enum DeploymentResourceSyncStatus {
    Creating,
    Updating,
    Deleting,
    Ready,
    Error,
}

#[derive(Debug, Identifiable, Queryable, Serialize, DieselFilter, ToSchema)]
#[diesel(table_name = deployment_resources)]
#[pagination]
pub struct DeploymentResource {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    #[filter]
    pub type_id: Uuid,
    pub deployment_id: Option<Uuid>,
    pub name: String,
    pub exists: bool,
    pub props: serde_json::Value,
    pub sync_status: DeploymentResourceSyncStatus,
    pub sync_reason: Option<String>,
}

impl DeploymentResource {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(deployment_resources::table
            .get_results_async(pool())
            .await?)
    }

    pub async fn all_filtered(filters: DeploymentResourceFilters) -> DbResult<Paginated<Self>> {
        let mut conn = pool().get()?;
        let page = filters.page.unwrap_or(1);
        let per_page = filters.per_page.unwrap_or(DEFAULT_PAGE_SIZE);
        let (items, num_total) = tokio::task::spawn_blocking(move || {
            Self::filter(&filters)
                .paginate(Some(page))
                .per_page(Some(per_page))
                .load_and_count::<Self>(&mut conn)
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

    pub async fn find(id: Uuid) -> DbResult<Self> {
        Ok(deployment_resources::table
            .find(id)
            .get_result_async(pool())
            .await?)
    }

    pub async fn find_by_type(type_id: Uuid) -> DbResult<Vec<Self>> {
        Ok(deployment_resources::table
            .filter(deployment_resources::type_id.eq(type_id))
            .get_results_async(pool())
            .await?)
    }

    pub async fn find_of_type(type_id: Uuid, id: Uuid) -> DbResult<Self> {
        Ok(deployment_resources::table
            .filter(deployment_resources::type_id.eq(type_id))
            .find(id)
            .get_result_async(pool())
            .await?)
    }

    pub async fn without_sensitive_props(mut self) -> DbResult<Self> {
        if let Some(map) = self.props.as_object_mut() {
            let resource_type = DeploymentResourceType::find(self.type_id).await?;
            for input in resource_type.spec()?.values_ui.inputs {
                if input.sensitive {
                    map.remove(&input.id);
                }
            }
        }
        Ok(self)
    }

    pub async fn sync_to(&self, target: &ChartExtActionTarget) -> DbResult<String> {
        let deployment = match self.deployment_id {
            None => {
                warn!(
                    "Not syncing deployment resource {} because its deployment_id is None",
                    self.id
                );
                return Ok("".to_owned());
            }
            Some(deployment_id) => Deployment::find(deployment_id).await?,
        };
        target
            .call(&deployment, self)
            .await
            .map_err(|err| DbError::DeploymentResourceSyncError(self.name.clone(), err.to_string()))
    }

    pub async fn delete(&self) -> DbResult<()> {
        UpdateDeploymentResourceExists {
            exists: Some(false),
        }
        .save(self.id)
        .await?;
        Ok(())
    }
}

#[derive(Debug, Insertable, Deserialize, ToSchema)]
#[diesel(table_name = deployment_resources)]
pub struct NewDeploymentResource {
    pub id: Option<Uuid>,
    pub created_at: Option<DateTime<Utc>>,
    pub type_id: Uuid,
    pub deployment_id: Uuid,
    pub name: String,
    pub props: serde_json::Value,
    pub sync_status: Option<DeploymentResourceSyncStatus>,
}

impl NewDeploymentResource {
    pub async fn insert(self) -> DbResult<DeploymentResource> {
        Ok(diesel::insert_into(deployment_resources::table)
            .values(self)
            .get_result_async(pool())
            .await?)
    }
}

#[derive(Debug, AsChangeset, Deserialize, ToSchema)]
#[diesel(table_name = deployment_resources)]
pub struct UpdateDeploymentResource {
    pub name: Option<String>,
    pub props: Option<serde_json::Value>,
}

// 🙏 https://github.com/serde-rs/json/issues/377#issuecomment-341490464

fn merge(a: &mut serde_json::Value, b: &serde_json::Value) {
    match (a, b) {
        (&mut serde_json::Value::Object(ref mut a), serde_json::Value::Object(b)) => {
            for (k, v) in b {
                merge(a.entry(k.clone()).or_insert(serde_json::Value::Null), v);
            }
        }
        (a, b) => {
            *a = b.clone();
        }
    }
}

impl UpdateDeploymentResource {
    pub async fn save(self, id: Uuid) -> DbResult<DeploymentResource> {
        let props = match self.props {
            None => None,
            Some(updates) => {
                let current = DeploymentResource::find(id).await?;
                let mut props = current.props;
                merge(&mut props, &updates);
                Some(props)
            }
        };
        Ok(
            diesel::update(deployment_resources::table.filter(deployment_resources::id.eq(id)))
                .set((
                    self.name.map(|name| deployment_resources::name.eq(name)),
                    props.map(|props| deployment_resources::props.eq(props)),
                ))
                .get_result_async(pool())
                .await?,
        )
    }
}

#[derive(Debug, AsChangeset, Deserialize, ToSchema)]
#[diesel(table_name = deployment_resources)]
pub struct UpdateDeploymentResourceExists {
    pub exists: Option<bool>,
}

impl UpdateDeploymentResourceExists {
    pub async fn save(self, id: Uuid) -> DbResult<DeploymentResource> {
        Ok(
            diesel::update(deployment_resources::table.filter(deployment_resources::id.eq(id)))
                .set(self)
                .get_result_async(pool())
                .await?,
        )
    }
}

#[derive(Debug, AsChangeset, Deserialize)]
#[diesel(table_name = deployment_resources)]
pub struct UpdateDeploymentResourceSyncStatus {
    pub sync_status: Option<DeploymentResourceSyncStatus>,
    pub sync_reason: Option<Option<String>>,
}

impl UpdateDeploymentResourceSyncStatus {
    pub async fn save(self, id: Uuid) -> DbResult<DeploymentResource> {
        Ok(
            diesel::update(deployment_resources::table.filter(deployment_resources::id.eq(id)))
                .set(self)
                .get_result_async(pool())
                .await?,
        )
    }
}
