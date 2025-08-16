use crate::{DbResult, db_conn};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use diesel_filter::DieselFilter;
use diesel_pagination::{Paginate, Paginated, PaginationParams};
use serde::Serialize;
use std::ops::DerefMut;
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
    pub async fn all_filtered(
        filters: BotTokenFilters,
        pagination: PaginationParams,
    ) -> DbResult<Paginated<Self>> {
        Ok(Self::filter(filters)
            .paginate(pagination)
            .load_and_count(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn find(id: Uuid) -> DbResult<Option<Self>> {
        Ok(bot_tokens::table
            .find(id)
            .get_result(db_conn().await?.deref_mut())
            .await
            .optional()?)
    }

    pub async fn delete(&self) -> DbResult<()> {
        diesel::delete(bot_tokens::table.find(self.id))
            .execute(db_conn().await?.deref_mut())
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
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }
}
