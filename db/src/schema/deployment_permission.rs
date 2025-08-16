use crate::{DbResult, db_conn};
use chrono::prelude::*;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use diesel_enum_derive::DieselEnum;
use diesel_filter::DieselFilter;
use diesel_pagination::{Paginate, Paginated, PaginationParams};
use serde::{Deserialize, Serialize};
use std::ops::DerefMut;
use strum::{Display, EnumString};
use utoipa::ToSchema;
use uuid::Uuid;

table! {
    deployment_permissions(id) {
        id -> Uuid,
        created_at -> Timestamptz,
        env_id -> Uuid,
        user_id -> Uuid,
        kind_id -> Uuid,
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
    DieselEnum,
    ToSchema,
)]
pub enum UserDeploymentRole {
    /// Deployment owners may perform any operation on the deployment kind,
    /// including creating and deleting deployments.
    Owner,
    /// Maintainers can edit deployments but not create or delete them.
    Maintainer,
}

#[derive(Debug, Identifiable, Queryable, Serialize, DieselFilter, ToSchema)]
#[diesel(table_name = deployment_permissions)]
pub struct DeploymentPermission {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    #[filter]
    pub env_id: Uuid,
    pub user_id: Uuid,
    pub kind_id: Uuid,
    pub role: UserDeploymentRole,
}

impl DeploymentPermission {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(deployment_permissions::table
            .get_results(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn all_filtered(
        filters: DeploymentPermissionFilters,
        pagination: PaginationParams,
    ) -> DbResult<Paginated<Self>> {
        Ok(Self::filter(filters)
            .paginate(pagination)
            .load_and_count(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn find(id: Uuid) -> DbResult<Self> {
        Ok(deployment_permissions::table
            .find(id)
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn find_user_role(
        env_id: Uuid,
        user_id: Uuid,
        kind_id: Uuid,
    ) -> DbResult<Option<UserDeploymentRole>> {
        Ok(deployment_permissions::table
            .filter(deployment_permissions::env_id.eq(env_id))
            .filter(deployment_permissions::user_id.eq(user_id))
            .filter(deployment_permissions::kind_id.eq(kind_id))
            .get_result::<Self>(db_conn().await?.deref_mut())
            .await
            .optional()?
            .map(|p| p.role))
    }

    pub async fn delete(&self) -> DbResult<()> {
        diesel::delete(deployment_permissions::table.find(self.id))
            .execute(db_conn().await?.deref_mut())
            .await?;
        Ok(())
    }
}

#[derive(Insertable, Deserialize, ToSchema)]
#[diesel(table_name = deployment_permissions)]
pub struct NewDeploymentPermission {
    pub env_id: Uuid,
    pub user_id: Uuid,
    pub kind_id: Uuid,
    pub role: UserDeploymentRole,
}

impl NewDeploymentPermission {
    pub async fn insert(self) -> DbResult<DeploymentPermission> {
        Ok(diesel::insert_into(deployment_permissions::table)
            .values(self)
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }
}
