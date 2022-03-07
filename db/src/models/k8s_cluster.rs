use crate::pool;
use crate::DbResult;
use async_diesel::*;
use chrono::prelude::*;
use diesel::prelude::*;
use diesel::QueryDsl;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

table! {
    k8s_clusters(id) {
        id -> Uuid,
        env_id -> Nullable<Uuid>,
        provider_id -> Varchar,
        created_at -> Timestamptz,
        last_seen_at -> Timestamptz,
        name -> Varchar,
        region_name -> Varchar,
        is_ok -> Bool,
        not_ok_reason -> Nullable<Varchar>,
        ignore -> Bool,
        domain -> Nullable<Varchar>,
    }
}

#[derive(Debug, Identifiable, Queryable, Serialize)]
pub struct K8sCluster {
    pub id: Uuid,
    pub env_id: Option<Uuid>,
    pub provider_id: String,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub name: String,
    pub region_name: String,
    pub is_ok: bool,
    pub not_ok_reason: Option<String>,
    pub ignore: bool,
    pub domain: Option<String>,
}

impl K8sCluster {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(k8s_clusters::table.get_results_async(pool()).await?)
    }

    pub async fn find(id: Uuid) -> DbResult<Self> {
        Ok(k8s_clusters::table
            .find(id)
            .get_result_async(pool())
            .await?)
    }

    pub async fn find_by_env_id(env_id: Uuid) -> DbResult<Vec<Self>> {
        Ok(k8s_clusters::table
            .filter(k8s_clusters::env_id.eq(env_id))
            .get_results_async(pool())
            .await?)
    }

    pub async fn find_by_provider_id(value: String) -> DbResult<Option<Self>> {
        Ok(k8s_clusters::table
            .filter(k8s_clusters::provider_id.eq(value))
            .first_async(pool())
            .await
            .optional()?)
    }

    pub async fn delete(&self) -> DbResult<()> {
        diesel::delete(k8s_clusters::table.find(self.id))
            .execute_async(pool())
            .await?;
        Ok(())
    }
}

#[derive(Debug, Insertable, Deserialize)]
#[table_name = "k8s_clusters"]
pub struct NewK8sCluster {
    pub provider_id: String,
    pub name: String,
    pub env_id: Option<Uuid>,
    pub region_name: String,
}

impl NewK8sCluster {
    pub async fn insert(self) -> DbResult<K8sCluster> {
        let name = self.name.clone();
        let region_name = self.region_name.clone();
        Ok(diesel::insert_into(k8s_clusters::table)
            .values(self)
            .on_conflict(k8s_clusters::provider_id)
            .do_update()
            .set((
                k8s_clusters::last_seen_at.eq(Utc::now()),
                k8s_clusters::name.eq(name),
                k8s_clusters::region_name.eq(region_name),
            ))
            .get_result_async(pool())
            .await?)
    }
}

#[derive(Debug, AsChangeset)]
#[table_name = "k8s_clusters"]
pub struct UpdateK8sClusterStatus {
    pub is_ok: Option<bool>,
    pub not_ok_reason: Option<Option<String>>,
}

impl UpdateK8sClusterStatus {
    pub async fn save(self, id: Uuid) -> DbResult<K8sCluster> {
        Ok(
            diesel::update(k8s_clusters::table.filter(k8s_clusters::id.eq(id)))
                .set(self)
                .get_result_async(pool())
                .await?,
        )
    }
}

#[derive(Debug, AsChangeset, Deserialize)]
#[table_name = "k8s_clusters"]
pub struct UpdateK8sCluster {
    #[serde(default, with = "::serde_with::rust::double_option")]
    pub env_id: Option<Option<Uuid>>,
    pub ignore: Option<bool>,
    #[serde(default, with = "::serde_with::rust::double_option")]
    pub domain: Option<Option<String>>,
}

impl UpdateK8sCluster {
    pub async fn save(self, id: Uuid) -> DbResult<K8sCluster> {
        Ok(
            diesel::update(k8s_clusters::table.filter(k8s_clusters::id.eq(id)))
                .set(self)
                .get_result_async(pool())
                .await?,
        )
    }
}
