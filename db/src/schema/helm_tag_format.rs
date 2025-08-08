use crate::{db_conn, DbResult};
use chrono::prelude::*;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use diesel_filter::DieselFilter;
use diesel_pagination::{Paginate, Paginated, PaginationParams};
use serde::{Deserialize, Serialize};
use std::ops::DerefMut;
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
            .get_results(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn all_filtered(
        filters: HelmTagFormatFilters,
        pagination: PaginationParams,
    ) -> DbResult<Paginated<Self>> {
        Ok(Self::filter(filters)
            .order_by(helm_tag_formats::created_at.desc())
            .paginate(pagination)
            .load_and_count(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn find(id: Uuid) -> DbResult<Self> {
        Ok(helm_tag_formats::table
            .find(id)
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn delete(&self) -> DbResult<()> {
        diesel::delete(helm_tag_formats::table.find(self.id))
            .execute(db_conn().await?.deref_mut())
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
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }
}
