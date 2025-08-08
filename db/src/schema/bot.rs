use crate::{db_conn, DbResult};
use chrono::prelude::*;
use diesel::{prelude::*, QueryDsl};
use diesel_async::RunQueryDsl;
use diesel_filter::DieselFilter;
use diesel_pagination::{Paginate, Paginated, PaginationParams};
use serde::{Deserialize, Serialize};
use std::ops::DerefMut;
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
pub struct Bot {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    #[filter(insensitive, substring)]
    pub display_name: String,
}

impl Bot {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(bots::table
            .get_results(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn all_filtered(
        filters: BotFilters,
        pagination: PaginationParams,
    ) -> DbResult<Paginated<Self>> {
        Ok(Self::filter(filters)
            .paginate(pagination)
            .load_and_count(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn find(id: Uuid) -> DbResult<Option<Self>> {
        Ok(bots::table
            .find(id)
            .get_result(db_conn().await?.deref_mut())
            .await
            .optional()?)
    }

    pub async fn delete(&self) -> DbResult<()> {
        diesel::delete(bots::table.find(self.id))
            .execute(db_conn().await?.deref_mut())
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
            .get_result(db_conn().await?.deref_mut())
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
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }
}
