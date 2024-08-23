use crate::{pool, DbResult, Paginated, DEFAULT_PAGE_SIZE};
use crate::{Env, EnvUserRole, NewEnvUserPermission};
use async_diesel::*;
use chrono::prelude::*;
use diesel::prelude::*;
use diesel::QueryDsl;
use diesel_filter::{DieselFilter, Paginate};
use serde::{Deserialize, Serialize};
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
#[pagination]
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
        Ok(users::table.get_results_async(pool()).await?)
    }

    pub async fn all_filtered(filters: UserFilters) -> DbResult<Paginated<Self>> {
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

    pub async fn find_only_active(id: Uuid) -> DbResult<Option<Self>> {
        Ok(users::table
            .filter(users::id.eq(id))
            .filter(users::is_active.eq(true))
            .get_result_async(pool())
            .await
            .optional()?)
    }

    pub async fn find(id: Uuid) -> DbResult<Option<Self>> {
        Ok(users::table
            .find(id)
            .get_result_async(pool())
            .await
            .optional()?)
    }

    pub async fn find_by_email(email: &str) -> DbResult<Option<Self>> {
        let email = email.to_owned();
        Ok(users::table
            .filter(users::email.eq(email))
            .get_result_async(pool())
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
            .get_result_async(pool())
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

#[derive(Debug, AsChangeset, Deserialize, ToSchema)]
#[diesel(table_name = users)]
pub struct UpdateUser {
    pub is_admin: Option<bool>,
    pub is_active: Option<bool>,
}

impl UpdateUser {
    pub async fn save(self, id: Uuid) -> DbResult<User> {
        Ok(diesel::update(users::table.filter(users::id.eq(id)))
            .set(self)
            .get_result_async(pool())
            .await?)
    }
}
