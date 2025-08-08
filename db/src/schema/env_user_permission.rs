use crate::{db_conn, DbResult};
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
            .get_results(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn all_filtered(
        filters: EnvUserPermissionFilters,
        pagination: PaginationParams,
    ) -> DbResult<Paginated<Self>> {
        Ok(Self::filter(filters)
            .paginate(pagination)
            .load_and_count(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn find(id: Uuid) -> DbResult<Self> {
        Ok(env_user_permissions::table
            .find(id)
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn find_user_role_in_env(
        env_id: Uuid,
        user_id: Uuid,
    ) -> DbResult<Option<EnvUserRole>> {
        Ok(env_user_permissions::table
            .filter(env_user_permissions::env_id.eq(env_id))
            .filter(env_user_permissions::user_id.eq(user_id))
            .get_result::<Self>(db_conn().await?.deref_mut())
            .await
            .optional()?
            .map(|p| p.role))
    }

    pub async fn delete(&self) -> DbResult<()> {
        diesel::delete(env_user_permissions::table.find(self.id))
            .execute(db_conn().await?.deref_mut())
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
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }
}
