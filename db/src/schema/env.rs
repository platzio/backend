use super::k8s_cluster::K8sCluster;
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
    envs(id) {
        id -> Uuid,
        created_at -> Timestamptz,
        name -> Varchar,
        node_selector -> Jsonb,
        tolerations -> Jsonb,
        auto_add_new_users -> Bool,
    }
}

#[derive(Debug, Identifiable, Queryable, Serialize, DieselFilter, ToSchema)]
#[diesel(table_name = envs)]
pub struct Env {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    #[filter(insensitive)]
    pub name: String,
    pub node_selector: serde_json::Value,
    pub tolerations: serde_json::Value,
    #[filter]
    pub auto_add_new_users: bool,
}

impl Env {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(envs::table
            .get_results(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn all_filtered(
        filters: EnvFilters,
        pagination: PaginationParams,
    ) -> DbResult<Paginated<Self>> {
        Ok(Self::filter(filters)
            .paginate(pagination)
            .load_and_count(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn find(id: Uuid) -> DbResult<Self> {
        Ok(envs::table
            .find(id)
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn delete(&self) -> DbResult<()> {
        K8sCluster::detach_from_env(self.id).await?;
        diesel::delete(envs::table.find(self.id))
            .execute(db_conn().await?.deref_mut())
            .await?;
        Ok(())
    }
}

#[derive(Debug, Insertable, Deserialize, ToSchema)]
#[diesel(table_name = envs)]
pub struct NewEnv {
    pub name: String,
    #[serde(default)]
    pub auto_add_new_users: bool,
}

impl NewEnv {
    pub async fn save(self) -> DbResult<Env> {
        Ok(diesel::insert_into(envs::table)
            .values(self)
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }
}

#[derive(Debug, AsChangeset, Deserialize, ToSchema)]
#[diesel(table_name = envs)]
pub struct UpdateEnv {
    pub name: Option<String>,
    pub node_selector: Option<serde_json::Value>,
    pub tolerations: Option<serde_json::Value>,
    pub auto_add_new_users: Option<bool>,
}

impl UpdateEnv {
    pub async fn save(self, id: Uuid) -> DbResult<Env> {
        Ok(diesel::update(envs::table.filter(envs::id.eq(id)))
            .set(self)
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }
}
