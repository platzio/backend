use crate::{pool, DbResult, Paginated, DEFAULT_PAGE_SIZE};
use async_diesel::*;
use chrono::prelude::*;
use diesel::prelude::*;
use diesel::QueryDsl;
use diesel_filter::{DieselFilter, Paginate};
use serde::Serialize;
use uuid::Uuid;

table! {
    k8s_resources(id) {
        id -> Uuid,
        last_updated_at -> Timestamptz,
        cluster_id -> Uuid,
        deployment_id -> Uuid,
        kind -> Varchar,
        api_version -> Varchar,
        name -> Varchar,
        status_color -> Array<Varchar>,
        metadata -> Jsonb,
    }
}

#[derive(Debug, Identifiable, Queryable, Insertable, Serialize, DieselFilter)]
#[diesel(table_name = k8s_resources)]
#[pagination]
pub struct K8sResource {
    pub id: Uuid,
    pub last_updated_at: DateTime<Utc>,
    #[filter]
    pub cluster_id: Uuid,
    #[filter]
    pub deployment_id: Uuid,
    #[filter(insensitive)]
    pub kind: String,
    pub api_version: String,
    #[filter(insensitive)]
    pub name: String,
    pub status_color: Vec<String>,
    pub metadata: serde_json::Value,
}

impl K8sResource {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(k8s_resources::table.get_results_async(pool()).await?)
    }

    pub async fn all_filtered(filters: K8sResourceFilters) -> DbResult<Paginated<Self>> {
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
        Ok(k8s_resources::table
            .find(id)
            .get_result_async(pool())
            .await
            .optional()?)
    }

    pub async fn save(self) -> DbResult<Self> {
        let deployment_id = self.deployment_id;
        let kind = self.kind.clone();
        let api_version = self.api_version.clone();
        let name = self.name.clone();
        let status_color = self.status_color.clone();
        let metadata = self.metadata.clone();
        let last_updated_at = self.last_updated_at;
        Ok(diesel::insert_into(k8s_resources::table)
            .values(self)
            .on_conflict(k8s_resources::id)
            .do_update()
            .set((
                k8s_resources::deployment_id.eq(deployment_id),
                k8s_resources::kind.eq(kind),
                k8s_resources::api_version.eq(api_version),
                k8s_resources::name.eq(name),
                k8s_resources::status_color.eq(status_color),
                k8s_resources::metadata.eq(metadata),
                k8s_resources::last_updated_at.eq(last_updated_at),
            ))
            .get_result_async(pool())
            .await?)
    }

    pub async fn find_older_than(
        cluster_id: Uuid,
        timestamp: DateTime<Utc>,
    ) -> DbResult<Vec<Self>> {
        Ok(k8s_resources::table
            .filter(k8s_resources::cluster_id.eq(cluster_id))
            .filter(k8s_resources::last_updated_at.lt(timestamp))
            .get_results_async(pool())
            .await?)
    }

    pub async fn delete_by_id(id: Uuid) -> DbResult<()> {
        diesel::delete(k8s_resources::table.find(id))
            .execute_async(pool())
            .await?;
        Ok(())
    }

    pub async fn delete(&self) -> DbResult<()> {
        diesel::delete(k8s_resources::table.find(self.id))
            .execute_async(pool())
            .await?;
        Ok(())
    }
}
