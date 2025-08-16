use crate::{DbResult, db_conn};
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

#[derive(Debug, Identifiable, Queryable, Insertable, Serialize, DieselFilter, ToSchema)]
#[diesel(table_name = k8s_resources)]
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
        Ok(k8s_resources::table
            .get_results(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn all_filtered(
        filters: K8sResourceFilters,
        pagination: PaginationParams,
    ) -> DbResult<Paginated<Self>> {
        Ok(Self::filter(filters)
            .paginate(pagination)
            .load_and_count(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn find(id: Uuid) -> DbResult<Option<Self>> {
        Ok(k8s_resources::table
            .find(id)
            .get_result(db_conn().await?.deref_mut())
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
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn find_older_than(
        cluster_id: Uuid,
        timestamp: DateTime<Utc>,
    ) -> DbResult<Vec<Self>> {
        Ok(k8s_resources::table
            .filter(k8s_resources::cluster_id.eq(cluster_id))
            .filter(k8s_resources::last_updated_at.lt(timestamp))
            .get_results(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn delete_by_id(id: Uuid) -> DbResult<()> {
        diesel::delete(k8s_resources::table.find(id))
            .execute(db_conn().await?.deref_mut())
            .await?;
        Ok(())
    }

    pub async fn delete(&self) -> DbResult<()> {
        diesel::delete(k8s_resources::table.find(self.id))
            .execute(db_conn().await?.deref_mut())
            .await?;
        Ok(())
    }
}
