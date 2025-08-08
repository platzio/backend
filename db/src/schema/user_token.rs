use crate::{db_conn, DbResult};
use chrono::prelude::*;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use diesel_filter::DieselFilter;
use diesel_pagination::{Paginate, Paginated, PaginationParams};
use serde::Serialize;
use std::ops::DerefMut;
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
pub struct UserToken {
    pub id: Uuid,
    #[filter]
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    #[serde(skip)]
    pub secret_hash: String,
}

impl UserToken {
    pub async fn all_filtered(
        filters: UserTokenFilters,
        pagination: PaginationParams,
    ) -> DbResult<Paginated<Self>> {
        Ok(Self::filter(filters)
            .paginate(pagination)
            .load_and_count(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn find(id: Uuid) -> DbResult<Option<Self>> {
        Ok(user_tokens::table
            .find(id)
            .get_result(db_conn().await?.deref_mut())
            .await
            .optional()?)
    }

    pub async fn delete(&self) -> DbResult<()> {
        diesel::delete(user_tokens::table.find(self.id))
            .execute(db_conn().await?.deref_mut())
            .await?;
        Ok(())
    }
}

#[derive(Insertable)]
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
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }
}
