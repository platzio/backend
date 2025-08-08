use super::deployment_kind::DeploymentKind;
use crate::{db_conn, DbError, DbResult, DbTableOrDeploymentResource};
use chrono::prelude::*;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use diesel_filter::DieselFilter;
use diesel_pagination::{Paginate, Paginated, PaginationParams};
use platz_chart_ext::resource_types::ChartExtResourceTypeV1Beta1Spec;
use serde::{Deserialize, Serialize};
use std::ops::DerefMut;
use utoipa::ToSchema;
use uuid::Uuid;

table! {
    deployment_resource_types(id) {
        id -> Uuid,
        created_at -> Timestamptz,
        env_id -> Nullable<Uuid>,
        deployment_kind_id -> Uuid,
        key -> Varchar,
        spec -> Jsonb,
    }
}

#[derive(Debug, Identifiable, Queryable, Serialize, DieselFilter, ToSchema)]
#[diesel(table_name = deployment_resource_types)]
pub struct DeploymentResourceType {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    #[filter]
    #[schema(required)]
    pub env_id: Option<Uuid>,
    #[filter]
    pub deployment_kind_id: Uuid,
    #[filter]
    pub key: String,
    #[schema(value_type = ChartExtResourceTypeV1Beta1Spec)]
    pub spec: serde_json::Value,
}

impl DeploymentResourceType {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(deployment_resource_types::table
            .get_results(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn all_filtered(
        filters: DeploymentResourceTypeFilters,
        pagination: PaginationParams,
    ) -> DbResult<Paginated<Self>> {
        Ok(Self::filter(filters)
            .paginate(pagination)
            .load_and_count(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn find(id: Uuid) -> DbResult<Self> {
        Ok(deployment_resource_types::table
            .find(id)
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn find_by_env(env_id: Uuid) -> DbResult<Self> {
        Ok(deployment_resource_types::table
            .filter(
                deployment_resource_types::env_id
                    .eq(env_id)
                    .or(deployment_resource_types::env_id.is_null()),
            )
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn find_by_env_kind_and_key(
        env_id: Uuid,
        deployment_kind: String,
        key: String,
    ) -> DbResult<Self> {
        let kind_obj = DeploymentKind::find_by_name(deployment_kind).await?;
        Ok(deployment_resource_types::table
            .filter(deployment_resource_types::deployment_kind_id.eq(kind_obj.id))
            .filter(deployment_resource_types::key.eq(key))
            .filter(
                deployment_resource_types::env_id
                    .eq(env_id)
                    .or(deployment_resource_types::env_id.is_null()),
            )
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn find_by_kind_and_key(deployment_kind: String, key: String) -> DbResult<Self> {
        let kind_obj = DeploymentKind::find_by_name(deployment_kind).await?;
        Ok(deployment_resource_types::table
            .filter(deployment_resource_types::deployment_kind_id.eq(kind_obj.id))
            .filter(deployment_resource_types::key.eq(key))
            .get_result(db_conn().await?.deref_mut())
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
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }

    pub fn spec(&self) -> DbResult<ChartExtResourceTypeV1Beta1Spec> {
        serde_json::from_value(self.spec.clone()).map_err(DbError::HelmChartResourceTypesParseError)
    }

    pub async fn as_db_collection(&self) -> DbTableOrDeploymentResource {
        let kind_obj = DeploymentKind::find(self.deployment_kind_id).await.unwrap();
        DbTableOrDeploymentResource::DeploymentResourceType {
            deployment: kind_obj.name,
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
    #[schema(required)]
    pub env_id: Option<Uuid>,
    pub deployment_kind_id: Uuid,
    pub key: String,
    pub spec: serde_json::Value,
}

impl NewDeploymentResourceType {
    pub async fn save(self) -> DbResult<()> {
        diesel::insert_into(deployment_resource_types::table)
            .values(self)
            .execute(db_conn().await?.deref_mut())
            .await?;
        Ok(())
    }
}
