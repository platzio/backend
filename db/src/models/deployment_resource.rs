use crate::pool;
use crate::DbError;
use crate::DbResult;
use crate::Deployment;
use crate::DeploymentResourceType;
use async_diesel::*;
use chrono::prelude::*;
use diesel::prelude::*;
use log::*;
use platz_chart_ext::ChartExtActionTarget;
use serde::{Deserialize, Serialize};
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
    }
}

#[derive(Debug, Identifiable, Queryable, Serialize)]
pub struct DeploymentResource {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub type_id: Uuid,
    pub deployment_id: Option<Uuid>,
    pub name: String,
    pub exists: bool,
    pub props: serde_json::Value,
}

impl DeploymentResource {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(deployment_resources::table
            .get_results_async(pool())
            .await?)
    }

    pub async fn find(id: Uuid) -> DbResult<Self> {
        Ok(deployment_resources::table
            .find(id)
            .get_result_async(pool())
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

    pub async fn sync_to(&self, target: &ChartExtActionTarget) -> DbResult<()> {
        let deployment = match self.deployment_id {
            None => {
                warn!(
                    "Not syncing deployment resource {} because its deployment_id is None",
                    self.id
                );
                return Ok(());
            }
            Some(deployment_id) => Deployment::find(deployment_id).await?,
        };
        target.call(&deployment, self).await.map_err(|err| {
            DbError::DeploymentResourceSyncError(self.name.clone(), err.to_string())
        })?;
        Ok(())
    }

    pub async fn delete(&self) -> DbResult<()> {
        diesel::delete(deployment_resources::table.find(self.id))
            .execute_async(pool())
            .await?;
        Ok(())
    }
}

#[derive(Debug, Insertable, Deserialize)]
#[table_name = "deployment_resources"]
pub struct NewDeploymentResource {
    pub id: Option<Uuid>,
    pub created_at: Option<DateTime<Utc>>,
    pub type_id: Uuid,
    pub deployment_id: Uuid,
    pub name: String,
    pub props: serde_json::Value,
}

impl NewDeploymentResource {
    pub async fn insert(self) -> DbResult<DeploymentResource> {
        Ok(diesel::insert_into(deployment_resources::table)
            .values(self)
            .get_result_async(pool())
            .await?)
    }
}

#[derive(Debug, AsChangeset, Deserialize)]
#[table_name = "deployment_resources"]
pub struct UpdateDeploymentResource {
    pub name: Option<String>,
    pub props: Option<serde_json::Value>,
}

impl UpdateDeploymentResource {
    pub async fn save(self, id: Uuid) -> DbResult<DeploymentResource> {
        Ok(
            diesel::update(deployment_resources::table.filter(deployment_resources::id.eq(id)))
                .set(self)
                .get_result_async(pool())
                .await?,
        )
    }
}
