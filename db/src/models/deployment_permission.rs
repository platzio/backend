use super::DeploymentKind;
use crate::pool;
use crate::DbResult;
use async_diesel::*;
use chrono::prelude::*;
use diesel::prelude::*;
use diesel_derive_more::DBEnum;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use uuid::Uuid;

table! {
    deployment_permissions(id) {
        id -> Uuid,
        created_at -> Timestamptz,
        env_id -> Uuid,
        user_id -> Uuid,
        kind -> Varchar,
        role -> Varchar,
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
pub enum UserDeploymentRole {
    Owner,
    Maintainer,
}

#[derive(Debug, Identifiable, Queryable, Serialize)]
pub struct DeploymentPermission {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub env_id: Uuid,
    pub user_id: Uuid,
    pub kind: DeploymentKind,
    pub role: UserDeploymentRole,
}

impl DeploymentPermission {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(deployment_permissions::table
            .get_results_async(pool())
            .await?)
    }

    pub async fn find(id: Uuid) -> DbResult<Self> {
        Ok(deployment_permissions::table
            .find(id)
            .get_result_async(pool())
            .await?)
    }

    pub async fn find_user_role(
        env_id: Uuid,
        user_id: Uuid,
        kind: DeploymentKind,
    ) -> DbResult<Option<UserDeploymentRole>> {
        Ok(deployment_permissions::table
            .filter(deployment_permissions::env_id.eq(env_id))
            .filter(deployment_permissions::user_id.eq(user_id))
            .filter(deployment_permissions::kind.eq(kind))
            .get_result_async::<Self>(pool())
            .await
            .optional()?
            .map(|p| p.role))
    }

    pub async fn delete(&self) -> DbResult<()> {
        diesel::delete(deployment_permissions::table.find(self.id))
            .execute_async(pool())
            .await?;
        Ok(())
    }
}

#[derive(Insertable, Deserialize)]
#[table_name = "deployment_permissions"]
pub struct NewDeploymentPermission {
    pub env_id: Uuid,
    pub user_id: Uuid,
    pub kind: DeploymentKind,
    pub role: UserDeploymentRole,
}

impl NewDeploymentPermission {
    pub async fn insert(self) -> DbResult<DeploymentPermission> {
        Ok(diesel::insert_into(deployment_permissions::table)
            .values(self)
            .get_result_async(pool())
            .await?)
    }
}
