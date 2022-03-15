use crate::{pool, DbTableOrDeploymentResource};
use crate::{DbError, DbResult};
use async_diesel::*;
use chrono::prelude::*;
use diesel::prelude::*;
use platz_chart_ext::resource_types::v1::ChartExtResourceTypeSpec;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Identifiable, Queryable, Serialize)]
pub struct DeploymentResourceType {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub env_id: Option<Uuid>,
    pub deployment_kind: String,
    pub key: String,
    pub spec: serde_json::Value,
}

impl DeploymentResourceType {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(deployment_resource_types::table
            .get_results_async(pool())
            .await?)
    }

    pub async fn find(id: Uuid) -> DbResult<Self> {
        Ok(deployment_resource_types::table
            .find(id)
            .get_result_async(pool())
            .await?)
    }

    pub async fn find_by_kind_and_key(
        env_id: Uuid,
        deployment_kind: &str,
        key: &str,
    ) -> DbResult<Self> {
        let key = key.to_owned();
        let deployment_kind = deployment_kind.to_owned();
        Ok(deployment_resource_types::table
            .filter(deployment_resource_types::key.eq(key))
            .filter(deployment_resource_types::deployment_kind.eq(deployment_kind))
            .filter(
                deployment_resource_types::env_id
                    .eq(env_id)
                    .or(deployment_resource_types::env_id.is_null()),
            )
            .get_result_async(pool())
            .await?)
    }

    pub async fn find_all_by_key(env_id: Uuid, key: &str) -> DbResult<Self> {
        let key = key.to_owned();
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

#[derive(Debug, Insertable, Deserialize)]
#[table_name = "deployment_resource_types"]
pub struct NewDeploymentResourceType {
    pub env_id: Option<Uuid>,
    pub deployment_kind: String,
    pub key: String,
    pub spec: serde_json::Value,
}

impl NewDeploymentResourceType {
    pub async fn save(self) -> DbResult<DeploymentResourceType> {
        let spec = self.spec.clone();
        Ok(diesel::insert_into(deployment_resource_types::table)
            .values(self)
            .on_conflict((
                deployment_resource_types::env_id,
                deployment_resource_types::deployment_kind,
                deployment_resource_types::key,
            ))
            .do_update()
            .set(deployment_resource_types::spec.eq(spec))
            .get_result_async(pool())
            .await?)
    }
}
