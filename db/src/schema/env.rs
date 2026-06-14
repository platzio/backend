use super::{deployment::Deployment, k8s_cluster::K8sCluster};
use crate::{AccessScope, DbResult, db_conn};
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

/// An environment together with its live deployment count, as returned by the
/// env list/detail endpoints. The count updates live on the frontend because a
/// deployment change emits an `envs` refresh event.
#[derive(Debug, Serialize, ToSchema)]
pub struct EnvWithStats {
    #[serde(flatten)]
    pub env: Env,
    pub num_deployments: i64,
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
        scope: &AccessScope,
    ) -> DbResult<Paginated<Self>> {
        let mut filtered = Self::filter(filters);
        if let AccessScope::Envs(env_ids) = scope {
            filtered = filtered.filter(envs::id.eq_any(env_ids.clone()));
        }
        Ok(filtered
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

    /// Like [`Self::find`] but only returns the env if it is within the
    /// identity's [`AccessScope`]. Out-of-scope and missing both yield
    /// `NotFound`.
    pub async fn find_scoped(id: Uuid, scope: &AccessScope) -> DbResult<Self> {
        if !scope.allows_env(Some(id)) {
            return Err(crate::DbError::NotFound);
        }
        Self::find(id).await
    }

    /// Like [`Self::all_filtered`] but augments each env with its live
    /// deployment count. The count is kept current on the frontend by an `envs`
    /// refresh event emitted whenever a deployment changes (see the
    /// `env-deployment-count` migration).
    pub async fn all_filtered_with_stats(
        filters: EnvFilters,
        pagination: PaginationParams,
        scope: &AccessScope,
    ) -> DbResult<Paginated<EnvWithStats>> {
        let page = Self::all_filtered(filters, pagination, scope).await?;
        let counts = Deployment::count_by_env(scope).await?;
        Ok(Paginated {
            page: page.page,
            per_page: page.per_page,
            num_total: page.num_total,
            items: page
                .items
                .into_iter()
                .map(|env| EnvWithStats {
                    num_deployments: counts.get(&env.id).copied().unwrap_or(0),
                    env,
                })
                .collect(),
        })
    }

    /// Like [`Self::find_scoped`] but augments the env with its deployment count.
    pub async fn find_scoped_with_stats(id: Uuid, scope: &AccessScope) -> DbResult<EnvWithStats> {
        let env = Self::find_scoped(id, scope).await?;
        let num_deployments = Deployment::count_in_env(env.id).await?;
        Ok(EnvWithStats {
            env,
            num_deployments,
        })
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
