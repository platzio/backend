use crate::{pool, DbResult, K8sCluster, Paginated, DEFAULT_PAGE_SIZE};
use async_diesel::*;
use chrono::prelude::*;
use diesel::prelude::*;
use diesel_filter::{DieselFilter, Paginate};
use serde::{Deserialize, Serialize};
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
#[pagination]
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
        Ok(envs::table.get_results_async(pool()).await?)
    }

    pub async fn all_filtered(filters: EnvFilters) -> DbResult<Paginated<Self>> {
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
        Ok(envs::table.find(id).get_result_async(pool()).await?)
    }

    pub async fn delete(&self) -> DbResult<()> {
        K8sCluster::detach_from_env(self.id).await?;
        diesel::delete(envs::table.find(self.id))
            .execute_async(pool())
            .await?;
        Ok(())
    }
}

#[derive(Debug, Insertable, Deserialize, ToSchema)]
#[diesel(table_name = envs)]
pub struct NewEnv {
    pub name: String,
    pub auto_add_new_users: Option<bool>,
}

impl NewEnv {
    pub async fn save(self) -> DbResult<Env> {
        Ok(diesel::insert_into(envs::table)
            .values(self)
            .get_result_async(pool())
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
            .get_result_async(pool())
            .await?)
    }
}
