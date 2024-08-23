use crate::{pool, DbResult, Paginated, DEFAULT_PAGE_SIZE};
use async_diesel::*;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::QueryDsl;
use diesel_filter::{DieselFilter, Paginate};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

table! {
    user_tokens(id) {
        id -> Uuid,
        user_id -> Uuid,
        created_at -> Timestamptz,
        secret_hash -> Varchar,
    }
}

#[derive(Debug, Identifiable, Queryable, Serialize, DieselFilter, ToSchema)]
#[diesel(table_name = user_tokens)]
#[pagination]
pub struct UserToken {
    pub id: Uuid,
    #[filter]
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    #[serde(skip)]
    pub secret_hash: String,
}

impl UserToken {
    pub async fn all_filtered(filters: UserTokenFilters) -> DbResult<Paginated<Self>> {
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
        Ok(user_tokens::table.find(id).get_result_async(pool()).await?)
    }

    pub async fn delete(&self) -> DbResult<()> {
        diesel::delete(user_tokens::table.find(self.id))
            .execute_async(pool())
            .await?;
        Ok(())
    }
}

#[derive(Deserialize, Insertable, ToSchema)]
#[diesel(table_name = user_tokens)]
pub struct NewUserToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub secret_hash: String,
}

impl NewUserToken {
    pub async fn save(self) -> DbResult<UserToken> {
        Ok(diesel::insert_into(user_tokens::table)
            .values(self)
            .get_result_async(pool())
            .await?)
    }
}
