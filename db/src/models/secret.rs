use crate::pool;
use crate::DbResult;
use async_diesel::*;
use chrono::prelude::*;
use diesel::prelude::*;
use diesel::QueryDsl;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

table! {
    secrets(id) {
        id -> Uuid,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        env_id -> Uuid,
        collection -> Varchar,
        name -> Varchar,
        contents -> Varchar,
    }
}

#[derive(Debug, Identifiable, Queryable, Serialize)]
pub struct Secret {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub env_id: Uuid,
    pub collection: String,
    pub name: String,
    #[serde(skip)]
    pub contents: String,
}

impl Secret {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(secrets::table.get_results_async(pool()).await?)
    }

    pub async fn find(id: Uuid) -> DbResult<Self> {
        Ok(secrets::table.find(id).get_result_async(pool()).await?)
    }
}

#[derive(Debug, Insertable, Deserialize)]
#[table_name = "secrets"]
pub struct NewSecret {
    pub env_id: Uuid,
    pub collection: String,
    pub name: String,
    pub contents: String,
}

impl NewSecret {
    pub async fn insert(self) -> DbResult<Secret> {
        Ok(diesel::insert_into(secrets::table)
            .values(self)
            .get_result_async(pool())
            .await?)
    }
}

#[derive(Debug, AsChangeset, Deserialize)]
#[table_name = "secrets"]
pub struct UpdateSecret {
    updated_at: Option<DateTime<Utc>>,
    name: Option<String>,
    contents: Option<String>,
}

impl UpdateSecret {
    pub fn new(name: Option<String>, contents: Option<String>) -> Self {
        Self {
            updated_at: Some(Utc::now()),
            name,
            contents,
        }
    }
}

impl UpdateSecret {
    pub async fn save(self, id: Uuid) -> DbResult<Secret> {
        Ok(diesel::update(secrets::table.filter(secrets::id.eq(id)))
            .set(self)
            .get_result_async(pool())
            .await?)
    }
}
