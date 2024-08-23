use crate::{pool, DbResult, Paginated, DEFAULT_PAGE_SIZE};
use async_diesel::*;
use chrono::prelude::*;
use diesel::prelude::*;
use diesel_enum_derive::DieselEnum;
use diesel_filter::{DieselFilter, Paginate};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use utoipa::ToSchema;
use uuid::Uuid;

table! {
    deployment_permissions(id) {
        id -> Uuid,
        created_at -> Timestamptz,
        env_id -> Uuid,
        user_id -> Uuid,
        kind -> Varchar,
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
#[pagination]
pub struct DeploymentPermission {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    #[filter]
    pub env_id: Uuid,
    pub user_id: Uuid,
    pub kind: String,
    pub kind_id: Uuid,
    pub role: UserDeploymentRole,
}

impl DeploymentPermission {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(deployment_permissions::table
            .get_results_async(pool())
            .await?)
    }

    pub async fn all_filtered(filters: DeploymentPermissionFilters) -> DbResult<Paginated<Self>> {
        let mut conn = pool().get()?;
        let page = filters.page.unwrap_or(1);
        let per_page = filters.per_page.unwrap_or(DEFAULT_PAGE_SIZE);
        let (items, num_total) = tokio::task::spawn_blocking(move || {
            Self::filter(&filters)
                .paginate(Some(page))
                .per_page(Some(per_page))
                .load_and_count::<Self>(&mut conn)
        })
        .await??;
        Ok(Paginated {
            page,
            per_page,
            num_total,
            items,
        })
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
        kind_id: Uuid,
    ) -> DbResult<Option<UserDeploymentRole>> {
        Ok(deployment_permissions::table
            .filter(deployment_permissions::env_id.eq(env_id))
            .filter(deployment_permissions::user_id.eq(user_id))
            .filter(deployment_permissions::kind_id.eq(kind_id))
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
            .get_result_async(pool())
            .await?)
    }
}
