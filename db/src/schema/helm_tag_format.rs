use crate::{pool, DbResult, Paginated, DEFAULT_PAGE_SIZE};
use async_diesel::*;
use chrono::prelude::*;
use diesel::prelude::*;
use diesel_filter::{DieselFilter, Paginate};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

table! {
    helm_tag_formats (id) {
        id -> Uuid,
        created_at -> Timestamptz,
        pattern -> Varchar,
    }
}

#[derive(Debug, Identifiable, Queryable, Serialize, DieselFilter, ToSchema)]
#[diesel(table_name = helm_tag_formats)]
#[pagination]
pub struct HelmTagFormat {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    #[filter]
    pub pattern: String,
}

impl HelmTagFormat {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(helm_tag_formats::table
            .order_by(helm_tag_formats::created_at.desc())
            .get_results_async(pool())
            .await?)
    }

    pub async fn all_filtered(filters: HelmTagFormatFilters) -> DbResult<Paginated<Self>> {
        let mut conn = pool().get()?;
        let page = filters.page.unwrap_or(1);
        let per_page = filters.per_page.unwrap_or(DEFAULT_PAGE_SIZE);
        let (items, num_total) = tokio::task::spawn_blocking(move || {
            Self::filter(&filters)
                .order_by(helm_tag_formats::created_at.desc())
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

    pub async fn find(id: Uuid) -> DbResult<Self> {
        Ok(helm_tag_formats::table
            .find(id)
            .get_result_async(pool())
            .await?)
    }

    pub async fn delete(&self) -> DbResult<()> {
        diesel::delete(helm_tag_formats::table.find(self.id))
            .execute_async(pool())
            .await?;
        Ok(())
    }
}

#[derive(Debug, Deserialize, Insertable, ToSchema)]
#[diesel(table_name = helm_tag_formats)]
pub struct NewHelmTagFormat {
    pub pattern: String,
}

impl NewHelmTagFormat {
    pub async fn insert(self) -> DbResult<HelmTagFormat> {
        Ok(diesel::insert_into(helm_tag_formats::table)
            .values(self)
            .get_result_async(pool())
            .await?)
    }
}
