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
    secrets(id) {
        id -> Uuid,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        env_id -> Uuid,
        collection -> Varchar,
        name -> Varchar,
        contents -> Varchar,
    }
}

#[derive(Debug, Identifiable, Queryable, Serialize, DieselFilter, ToSchema)]
#[diesel(table_name = secrets)]
#[pagination]
pub struct Secret {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[filter]
    pub env_id: Uuid,
    #[filter(insensitive)]
    pub collection: String,
    #[filter(insensitive)]
    pub name: String,
    #[serde(skip)]
    pub contents: String,
}

impl Secret {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(secrets::table.get_results_async(pool()).await?)
    }

    pub async fn all_filtered(filters: SecretFilters) -> DbResult<Paginated<Self>> {
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
        Ok(secrets::table.find(id).get_result_async(pool()).await?)
    }

    pub async fn delete(&self) -> DbResult<()> {
        diesel::delete(secrets::table.find(self.id))
            .execute_async(pool())
            .await?;
        Ok(())
    }
}

#[derive(Debug, Insertable, Deserialize, ToSchema)]
#[diesel(table_name = secrets)]
pub struct NewSecret {
    pub env_id: Uuid,
    pub collection: String,
    pub name: String,
    pub contents: String,
}

impl NewSecret {
    pub async fn insert(self) -> DbResult<Secret> {
        Ok(diesel::insert_into(secrets::table)
            .values(self)
            .get_result_async(pool())
            .await?)
    }
}

#[derive(Debug, AsChangeset, Deserialize, ToSchema)]
#[diesel(table_name = secrets)]
pub struct UpdateSecret {
    updated_at: Option<DateTime<Utc>>,
    name: Option<String>,
    contents: Option<String>,
}

impl UpdateSecret {
    pub fn new(name: Option<String>, contents: Option<String>) -> Self {
        Self {
            updated_at: Some(Utc::now()),
            name,
            contents,
        }
    }
}

impl UpdateSecret {
    pub async fn save(self, id: Uuid) -> DbResult<Secret> {
        Ok(diesel::update(secrets::table.filter(secrets::id.eq(id)))
            .set(self)
            .get_result_async(pool())
            .await?)
    }
}
