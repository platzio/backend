use crate::{pool, DbResult, Paginated, DEFAULT_PAGE_SIZE};
use async_diesel::*;
use chrono::prelude::*;
use diesel::prelude::*;
use diesel::QueryDsl;
use diesel_filter::{DieselFilter, Paginate};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

table! {
    bots(id) {
        id -> Uuid,
        created_at -> Timestamptz,
        display_name -> Varchar,
    }
}

#[derive(Debug, Identifiable, Queryable, Serialize, DieselFilter, ToSchema)]
#[diesel(table_name = bots)]
#[pagination]
pub struct Bot {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    #[filter(insensitive, substring)]
    pub display_name: String,
}

impl Bot {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(bots::table.get_results_async(pool()).await?)
    }

    pub async fn all_filtered(filters: BotFilters) -> DbResult<Paginated<Self>> {
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

    pub async fn find(id: Uuid) -> DbResult<Option<Self>> {
        Ok(bots::table
            .find(id)
            .get_result_async(pool())
            .await
            .optional()?)
    }

    pub async fn delete(&self) -> DbResult<()> {
        diesel::delete(bots::table.find(self.id))
            .execute_async(pool())
            .await?;
        Ok(())
    }
}

#[derive(Debug, Insertable, Deserialize, ToSchema)]
#[diesel(table_name = bots)]
pub struct NewBot {
    pub display_name: String,
}

impl NewBot {
    pub async fn insert(self) -> DbResult<Bot> {
        let bot: Bot = diesel::insert_into(bots::table)
            .values(self)
            .get_result_async(pool())
            .await?;
        Ok(bot)
    }
}

#[derive(Debug, AsChangeset, Deserialize, ToSchema)]
#[diesel(table_name = bots)]
pub struct UpdateBot {
    pub display_name: Option<String>,
}

impl UpdateBot {
    pub async fn save(self, id: Uuid) -> DbResult<Bot> {
        Ok(diesel::update(bots::table.find(id))
            .set(self)
            .get_result_async(pool())
            .await?)
    }
}
