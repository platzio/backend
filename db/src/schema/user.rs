use crate::{pool, DbResult, Paginated, DEFAULT_PAGE_SIZE};
use async_diesel::*;
use chrono::prelude::*;
use diesel::prelude::*;
use diesel::QueryDsl;
use diesel_filter::{DieselFilter, Paginate};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

table! {
    users(id) {
        id -> Uuid,
        created_at -> Timestamptz,
        display_name -> Varchar,
        email -> Varchar,
        is_admin -> Bool,
    }
}

#[derive(Debug, Identifiable, Queryable, Serialize, DieselFilter)]
#[table_name = "users"]
#[pagination]
pub struct User {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    #[filter(insensitive, substring)]
    pub display_name: String,
    #[filter(insensitive, substring)]
    pub email: String,
    pub is_admin: bool,
}

impl User {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(users::table.get_results_async(pool()).await?)
    }

    pub async fn all_filtered(filters: UserFilters) -> DbResult<Paginated<Self>> {
        let conn = pool().get()?;
        let page = filters.page.unwrap_or(1);
        let per_page = filters.per_page.unwrap_or(DEFAULT_PAGE_SIZE);
        let (items, num_total) = tokio::task::spawn_blocking(move || {
            Self::filter(&filters)
                .paginate(Some(page))
                .per_page(Some(per_page))
                .load_and_count::<Self>(&conn)
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

#[derive(Debug, Insertable, Deserialize)]
#[table_name = "users"]
pub struct NewUser {
    pub display_name: String,
    pub email: String,
}

impl NewUser {
    pub async fn insert(self) -> DbResult<User> {
        Ok(diesel::insert_into(users::table)
            .values(self)
            .get_result_async(pool())
            .await?)
    }
}

#[derive(Debug, AsChangeset, Deserialize)]
#[table_name = "users"]
pub struct UpdateUser {
    pub is_admin: Option<bool>,
}

impl UpdateUser {
    pub async fn save(self, id: Uuid) -> DbResult<User> {
        Ok(diesel::update(users::table.filter(users::id.eq(id)))
            .set(self)
            .get_result_async(pool())
            .await?)
    }
}