use crate::{DbResult, db_conn};
use chrono::prelude::*;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::Deserialize;
use std::ops::DerefMut;
use uuid::Uuid;

table! {
    settings(id) {
        id -> Uuid,
        created_at -> Timestamptz,
        key -> Varchar,
        value -> Varchar,
    }
}

#[derive(Debug, Identifiable, Queryable)]
pub struct Setting {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub key: String,
    pub value: String,
}

impl Setting {
    pub async fn get(key: &str) -> DbResult<Option<Self>> {
        Ok(settings::table
            .filter(settings::key.eq(key.to_owned()))
            .get_result(db_conn().await?.deref_mut())
            .await
            .optional()?)
    }

    pub async fn get_or_set_default<F>(key: &str, default: F) -> DbResult<Self>
    where
        F: FnOnce() -> String,
    {
        Ok(match Self::get(key).await? {
            Some(value) => value,
            None => {
                UpdateSetting {
                    key: key.to_owned(),
                    value: default(),
                }
                .insert()
                .await?
            }
        })
    }
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = settings)]
pub struct UpdateSetting {
    pub key: String,
    pub value: String,
}

impl UpdateSetting {
    async fn insert(self) -> DbResult<Setting> {
        Ok(diesel::insert_into(settings::table)
            .values(self)
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn save(self) -> DbResult<Setting> {
        let value = self.value.clone();
        Ok(diesel::insert_into(settings::table)
            .values(self)
            .on_conflict(settings::key)
            .do_update()
            .set(settings::value.eq(value))
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }
}
