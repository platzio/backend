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
    env_user_permissions(id) {
        id -> Uuid,
        created_at -> Timestamptz,
        env_id -> Uuid,
        user_id -> Uuid,
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
pub enum EnvUserRole {
    Admin,
    User,
}

#[derive(Debug, Identifiable, Queryable, Serialize, DieselFilter, ToSchema)]
#[diesel(table_name = env_user_permissions)]
#[pagination]
pub struct EnvUserPermission {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    #[filter]
    pub env_id: Uuid,
    pub user_id: Uuid,
    pub role: EnvUserRole,
}

impl EnvUserPermission {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(env_user_permissions::table
            .get_results_async(pool())
            .await?)
    }

    pub async fn all_filtered(filters: EnvUserPermissionFilters) -> DbResult<Paginated<Self>> {
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
        Ok(env_user_permissions::table
            .find(id)
            .get_result_async(pool())
            .await?)
    }

    pub async fn find_user_role_in_env(
        env_id: Uuid,
        user_id: Uuid,
    ) -> DbResult<Option<EnvUserRole>> {
        Ok(env_user_permissions::table
            .filter(env_user_permissions::env_id.eq(env_id))
            .filter(env_user_permissions::user_id.eq(user_id))
            .get_result_async::<Self>(pool())
            .await
            .optional()?
            .map(|p| p.role))
    }

    pub async fn delete(&self) -> DbResult<()> {
        diesel::delete(env_user_permissions::table.find(self.id))
            .execute_async(pool())
            .await?;
        Ok(())
    }
}

#[derive(Insertable, Deserialize, ToSchema)]
#[diesel(table_name = env_user_permissions)]
pub struct NewEnvUserPermission {
    pub env_id: Uuid,
    pub user_id: Uuid,
    pub role: EnvUserRole,
}

impl NewEnvUserPermission {
    pub async fn insert(self) -> DbResult<EnvUserPermission> {
        Ok(diesel::insert_into(env_user_permissions::table)
            .values(self)
            .get_result_async(pool())
            .await?)
    }
}
