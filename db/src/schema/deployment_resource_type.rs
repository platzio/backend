use crate::{pool, DbTableOrDeploymentResource, Paginated, DEFAULT_PAGE_SIZE};
use crate::{DbError, DbResult};
use async_diesel::*;
use chrono::prelude::*;
use diesel::prelude::*;
use diesel_filter::{DieselFilter, Paginate};
use platz_chart_ext::resource_types::v1beta1::ChartExtResourceTypeSpec;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

table! {
    deployment_resource_types(id) {
        id -> Uuid,
        created_at -> Timestamptz,
        env_id -> Nullable<Uuid>,
        deployment_kind -> VarChar,
        key -> Varchar,
        spec -> Jsonb,
    }
}

#[derive(Debug, Identifiable, Queryable, Serialize, DieselFilter, ToSchema)]
#[diesel(table_name = deployment_resource_types)]
#[pagination]
pub struct DeploymentResourceType {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    #[filter]
    pub env_id: Option<Uuid>,
    #[filter(insensitive)]
    pub deployment_kind: String,
    #[filter]
    pub key: String,
    pub spec: serde_json::Value,
}

impl DeploymentResourceType {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(deployment_resource_types::table
            .get_results_async(pool())
            .await?)
    }

    pub async fn all_filtered(filters: DeploymentResourceTypeFilters) -> DbResult<Paginated<Self>> {
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
        Ok(deployment_resource_types::table
            .find(id)
            .get_result_async(pool())
            .await?)
    }

    pub async fn find_by_env(env_id: Uuid) -> DbResult<Self> {
        Ok(deployment_resource_types::table
            .filter(
                deployment_resource_types::env_id
                    .eq(env_id)
                    .or(deployment_resource_types::env_id.is_null()),
            )
            .get_result_async(pool())
            .await?)
    }

    pub async fn find_by_env_kind_and_key(
        env_id: Uuid,
        deployment_kind: String,
        key: String,
    ) -> DbResult<Self> {
        Ok(deployment_resource_types::table
            .filter(deployment_resource_types::deployment_kind.eq(deployment_kind))
            .filter(deployment_resource_types::key.eq(key))
            .filter(
                deployment_resource_types::env_id
                    .eq(env_id)
                    .or(deployment_resource_types::env_id.is_null()),
            )
            .get_result_async(pool())
            .await?)
    }

    pub async fn find_by_kind_and_key(deployment_kind: String, key: String) -> DbResult<Self> {
        Ok(deployment_resource_types::table
            .filter(deployment_resource_types::deployment_kind.eq(deployment_kind))
            .filter(deployment_resource_types::key.eq(key))
            .get_result_async(pool())
            .await?)
    }

    pub async fn find_all_by_key(env_id: Uuid, key: String) -> DbResult<Self> {
        Ok(deployment_resource_types::table
            .filter(deployment_resource_types::key.eq(key))
            .filter(
                deployment_resource_types::env_id
                    .eq(env_id)
                    .or(deployment_resource_types::env_id.is_null()),
            )
            .get_result_async(pool())
            .await?)
    }

    pub fn spec(&self) -> DbResult<ChartExtResourceTypeSpec> {
        serde_json::from_value(self.spec.clone()).map_err(DbError::HelmChartResourceTypesParseError)
    }

    pub fn as_db_collection(&self) -> DbTableOrDeploymentResource {
        DbTableOrDeploymentResource::DeploymentResourceType {
            deployment: self.deployment_kind.clone(),
            r#type: self.key.clone(),
        }
    }

    pub fn as_legacy_db_collection(&self) -> DbTableOrDeploymentResource {
        DbTableOrDeploymentResource::LegacyCollectionName(self.key.clone())
    }
}

#[derive(Debug, Insertable, Deserialize, ToSchema)]
#[diesel(table_name = deployment_resource_types)]
pub struct NewDeploymentResourceType {
    pub env_id: Option<Uuid>,
    pub deployment_kind: String,
    pub key: String,
    pub spec: serde_json::Value,
}

impl NewDeploymentResourceType {
    pub async fn save(self) -> DbResult<()> {
        diesel::insert_into(deployment_resource_types::table)
            .values(self)
            .execute_async(pool())
            .await?;
        Ok(())
    }
}
