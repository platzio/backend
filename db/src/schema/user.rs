use super::{
    env::Env,
    env_user_permission::{EnvUserRole, NewEnvUserPermission},
};
use crate::{DbResult, db_conn};
use chrono::prelude::*;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use diesel_filter::DieselFilter;
use diesel_pagination::{Paginate, Paginated, PaginationParams};
use serde::{Deserialize, Serialize};
use std::ops::DerefMut;
use tracing::info;
use utoipa::ToSchema;
use uuid::Uuid;

table! {
    users(id) {
        id -> Uuid,
        created_at -> Timestamptz,
        display_name -> Varchar,
        email -> Varchar,
        is_admin -> Bool,
        is_active -> Bool,
    }
}

#[derive(Debug, Identifiable, Queryable, Serialize, DieselFilter, ToSchema)]
#[diesel(table_name = users)]
pub struct User {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    #[filter(insensitive, substring)]
    pub display_name: String,
    #[filter(insensitive, substring)]
    pub email: String,
    pub is_admin: bool,
    #[filter]
    pub is_active: bool,
}

impl User {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(users::table
            .get_results(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn all_filtered(
        filters: UserFilters,
        pagination: PaginationParams,
    ) -> DbResult<Paginated<Self>> {
        Ok(Self::filter(filters)
            .paginate(pagination)
            .load_and_count(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn find_only_active(id: Uuid) -> DbResult<Option<Self>> {
        Ok(users::table
            .filter(users::id.eq(id))
            .filter(users::is_active.eq(true))
            .get_result(db_conn().await?.deref_mut())
            .await
            .optional()?)
    }

    pub async fn find(id: Uuid) -> DbResult<Option<Self>> {
        Ok(users::table
            .find(id)
            .get_result(db_conn().await?.deref_mut())
            .await
            .optional()?)
    }

    pub async fn find_by_email(email: &str) -> DbResult<Option<Self>> {
        let email = email.to_owned();
        Ok(users::table
            .filter(users::email.eq(email))
            .get_result(db_conn().await?.deref_mut())
            .await
            .optional()?)
    }
}

#[derive(Debug, Insertable, Deserialize, ToSchema)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub display_name: String,
    pub email: String,
    pub is_admin: bool,
}

impl NewUser {
    pub async fn insert(self) -> DbResult<User> {
        let user: User = diesel::insert_into(users::table)
            .values(self)
            .get_result(db_conn().await?.deref_mut())
            .await?;
        for env in Env::all().await? {
            if env.auto_add_new_users {
                info!("Auto adding new user {:?} to env {:?}", user.id, env.id);
                NewEnvUserPermission {
                    env_id: env.id,
                    user_id: user.id,
                    role: EnvUserRole::User,
                }
                .insert()
                .await?;
            }
        }
        Ok(user)
    }
}

#[derive(AsChangeset, Deserialize, ToSchema)]
#[diesel(table_name = users)]
pub struct UpdateUser {
    pub is_admin: Option<bool>,
    pub is_active: Option<bool>,
}

impl UpdateUser {
    pub async fn save(self, id: Uuid) -> DbResult<User> {
        Ok(diesel::update(users::table.filter(users::id.eq(id)))
            .set(self)
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }
}
