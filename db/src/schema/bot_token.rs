use crate::{pool, DbResult, Paginated, DEFAULT_PAGE_SIZE};
use async_diesel::*;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::QueryDsl;
use diesel_filter::{DieselFilter, Paginate};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

table! {
    bot_tokens(id) {
        id -> Uuid,
        bot_id -> Uuid,
        created_at -> Timestamptz,
        created_by_user_id -> Uuid,
        secret_hash -> Varchar,
    }
}

#[derive(Debug, Identifiable, Queryable, Serialize, DieselFilter, ToSchema)]
#[diesel(table_name = bot_tokens)]
#[pagination]
pub struct BotToken {
    pub id: Uuid,
    #[filter]
    pub bot_id: Uuid,
    pub created_at: DateTime<Utc>,
    #[filter]
    pub created_by_user_id: Uuid,
    #[serde(skip)]
    pub secret_hash: String,
}

impl BotToken {
    pub async fn all_filtered(filters: BotTokenFilters) -> DbResult<Paginated<Self>> {
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

    pub async fn find(id: Uuid) -> DbResult<Option<Self>> {
        Ok(bot_tokens::table
            .find(id)
            .get_result_async(pool())
            .await
            .optional()?)
    }

    pub async fn delete(&self) -> DbResult<()> {
        diesel::delete(bot_tokens::table.find(self.id))
            .execute_async(pool())
            .await?;
        Ok(())
    }
}

#[derive(Insertable)]
#[diesel(table_name = bot_tokens)]
pub struct NewBotToken {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub created_by_user_id: Uuid,
    pub secret_hash: String,
}

impl NewBotToken {
    pub async fn save(self) -> DbResult<BotToken> {
        Ok(diesel::insert_into(bot_tokens::table)
            .values(self)
            .get_result_async(pool())
            .await?)
    }
}
