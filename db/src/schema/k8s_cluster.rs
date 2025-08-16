use crate::{DbResult, db_conn};
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
        ingress_domain -> Nullable<Varchar>,
        ingress_class -> Nullable<Varchar>,
        ingress_tls_secret_name -> Nullable<Varchar>,
        grafana_url -> Nullable<Varchar>,
        grafana_datasource_name -> Nullable<Varchar>,
    }
}

#[derive(Debug, Clone, Identifiable, Queryable, Serialize, DieselFilter, ToSchema)]
#[diesel(table_name = k8s_clusters)]
pub struct K8sCluster {
    pub id: Uuid,
    #[filter]
    #[schema(required)]
    pub env_id: Option<Uuid>,
    pub provider_id: String,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    #[filter(insensitive)]
    pub name: String,
    pub region_name: String,
    pub is_ok: bool,
    #[schema(required)]
    pub not_ok_reason: Option<String>,
    pub ignore: bool,
    #[schema(required)]
    pub ingress_domain: Option<String>,
    #[schema(required)]
    pub ingress_class: Option<String>,
    #[schema(required)]
    pub ingress_tls_secret_name: Option<String>,
    #[schema(required)]
    pub grafana_url: Option<String>,
    #[schema(required)]
    pub grafana_datasource_name: Option<String>,
}

impl K8sCluster {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(k8s_clusters::table
            .get_results(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn all_filtered(
        filters: K8sClusterFilters,
        pagination: PaginationParams,
    ) -> DbResult<Paginated<Self>> {
        Ok(Self::filter(filters)
            .paginate(pagination)
            .load_and_count(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn find(id: Uuid) -> DbResult<Self> {
        Ok(k8s_clusters::table
            .find(id)
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn find_by_env_id(env_id: Uuid) -> DbResult<Vec<Self>> {
        Ok(k8s_clusters::table
            .filter(k8s_clusters::env_id.eq(env_id))
            .get_results(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn find_by_provider_id(value: String) -> DbResult<Option<Self>> {
        Ok(k8s_clusters::table
            .filter(k8s_clusters::provider_id.eq(value))
            .first(db_conn().await?.deref_mut())
            .await
            .optional()?)
    }

    pub async fn delete(&self) -> DbResult<()> {
        diesel::delete(k8s_clusters::table.find(self.id))
            .execute(db_conn().await?.deref_mut())
            .await?;
        Ok(())
    }

    pub async fn detach_from_env(env_id: Uuid) -> DbResult<()> {
        diesel::update(k8s_clusters::table.filter(k8s_clusters::env_id.eq(env_id)))
            .set(UpdateK8sCluster {
                env_id: Some(None),
                ignore: None,
                ingress_class: None,
                ingress_domain: None,
                ingress_tls_secret_name: None,
                grafana_url: None,
                grafana_datasource_name: None,
            })
            .execute(db_conn().await?.deref_mut())
            .await?;
        Ok(())
    }
}

#[derive(Debug, Insertable, Deserialize, ToSchema)]
#[diesel(table_name = k8s_clusters)]
pub struct NewK8sCluster {
    pub provider_id: String,
    pub name: String,
    #[schema(required)]
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
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }
}

#[derive(AsChangeset)]
#[diesel(table_name = k8s_clusters)]
pub struct UpdateK8sClusterStatus {
    pub is_ok: Option<bool>,
    pub not_ok_reason: Option<Option<String>>,
}

impl UpdateK8sClusterStatus {
    pub async fn save(self, id: Uuid) -> DbResult<K8sCluster> {
        Ok(
            diesel::update(k8s_clusters::table.filter(k8s_clusters::id.eq(id)))
                .set(self)
                .get_result(db_conn().await?.deref_mut())
                .await?,
        )
    }
}

#[derive(Debug, AsChangeset, Deserialize, ToSchema)]
#[diesel(table_name = k8s_clusters)]
pub struct UpdateK8sCluster {
    #[serde(default, with = "::serde_with::rust::double_option")]
    pub env_id: Option<Option<Uuid>>,
    pub ignore: Option<bool>,
    #[serde(default, with = "::serde_with::rust::double_option")]
    pub ingress_domain: Option<Option<String>>,
    #[serde(default, with = "::serde_with::rust::double_option")]
    pub ingress_class: Option<Option<String>>,
    #[serde(default, with = "::serde_with::rust::double_option")]
    pub ingress_tls_secret_name: Option<Option<String>>,
    #[serde(default, with = "::serde_with::rust::double_option")]
    pub grafana_url: Option<Option<String>>,
    #[serde(default, with = "::serde_with::rust::double_option")]
    pub grafana_datasource_name: Option<Option<String>>,
}

impl UpdateK8sCluster {
    pub async fn save(self, id: Uuid) -> DbResult<K8sCluster> {
        Ok(
            diesel::update(k8s_clusters::table.filter(k8s_clusters::id.eq(id)))
                .set(self)
                .get_result(db_conn().await?.deref_mut())
                .await?,
        )
    }
}
