use crate::pool;
use crate::DbResult;
use async_diesel::*;
use chrono::prelude::*;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

table! {
    envs(id) {
        id -> Uuid,
        created_at -> Timestamptz,
        name -> Varchar,
        node_selector -> Jsonb,
        tolerations -> Jsonb,
    }
}

#[derive(Debug, Identifiable, Queryable, Serialize)]
pub struct Env {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub name: String,
    pub node_selector: serde_json::Value,
    pub tolerations: serde_json::Value,
}

impl Env {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(envs::table.get_results_async(pool()).await?)
    }

    pub async fn find(id: Uuid) -> DbResult<Self> {
        Ok(envs::table.find(id).get_result_async(pool()).await?)
    }
}

#[derive(Debug, Insertable, Deserialize)]
#[table_name = "envs"]
pub struct NewEnv {
    pub name: String,
}

impl NewEnv {
    pub async fn save(self) -> DbResult<Env> {
        Ok(diesel::insert_into(envs::table)
            .values(self)
            .get_result_async(pool())
            .await?)
    }
}

#[derive(Debug, AsChangeset, Deserialize)]
#[table_name = "envs"]
pub struct UpdateEnv {
    pub name: Option<String>,
    pub node_selector: Option<serde_json::Value>,
    pub tolerations: Option<serde_json::Value>,
}

impl UpdateEnv {
    pub async fn save(self, id: Uuid) -> DbResult<Env> {
        Ok(diesel::update(envs::table.filter(envs::id.eq(id)))
            .set(self)
            .get_result_async(pool())
            .await?)
    }
}
