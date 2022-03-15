use crate::pool;
use crate::DbError;
use crate::DbResult;
use async_diesel::*;
use chrono::prelude::*;
use diesel::prelude::*;
use diesel::QueryDsl;
pub use diesel_json::Json;
use platz_chart_ext::resource_types::ChartExtResourceTypes;
use platz_chart_ext::UiSchema;
use platz_chart_ext::{ChartExtActions, ChartExtFeatures};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

table! {
    helm_charts(id) {
        id -> Uuid,
        created_at -> Timestamptz,
        helm_registry_id -> Uuid,
        image_digest -> Varchar,
        image_tag -> Varchar,
        available -> Bool,
        values_ui -> Nullable<Jsonb>,
        actions_schema -> Nullable<Jsonb>,
        features -> Nullable<Jsonb>,
        resource_types -> Nullable<Jsonb>,
        error -> Nullable<Varchar>,
    }
}

#[derive(Debug, Identifiable, Queryable, Serialize)]
pub struct HelmChart {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub helm_registry_id: Uuid,
    pub image_digest: String,
    pub image_tag: String,
    pub available: bool,
    pub values_ui: Option<Json<UiSchema>>,
    pub actions_schema: Option<Json<ChartExtActions>>,
    pub features: Option<Json<ChartExtFeatures>>,
    pub resource_types: Option<Json<ChartExtResourceTypes>>,
    pub error: Option<String>,
}

impl HelmChart {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(helm_charts::table.get_results_async(pool()).await?)
    }

    pub async fn find(id: Uuid) -> DbResult<Self> {
        Ok(helm_charts::table.find(id).get_result_async(pool()).await?)
    }

    pub async fn find_by_registry_and_digest(
        registry_id: Uuid,
        image_digest: String,
    ) -> DbResult<Option<Self>> {
        Ok(helm_charts::table
            .filter(helm_charts::helm_registry_id.eq(registry_id))
            .filter(helm_charts::image_digest.eq(image_digest))
            .first_async(pool())
            .await
            .optional()?)
    }

    pub fn values_ui(&self) -> Option<&UiSchema> {
        match self.values_ui.as_ref() {
            Some(Json(values_ui)) => Some(values_ui),
            None => None,
        }
    }

    pub fn actions_schema(&self) -> DbResult<&ChartExtActions> {
        match self.actions_schema.as_ref() {
            Some(Json(actions)) => Ok(actions),
            None => Err(DbError::HelmChartNoActionsSchema),
        }
    }

    pub fn features(&self) -> DbResult<ChartExtFeatures> {
        Ok(match self.features.as_ref() {
            None => Default::default(),
            Some(Json(features)) => features.to_owned(),
        })
    }

    pub fn resource_types(&self) -> DbResult<ChartExtResourceTypes> {
        Ok(match self.resource_types.as_ref() {
            None => Default::default(),
            Some(Json(resource_types)) => resource_types.to_owned(),
        })
    }
}

#[derive(Insertable, Deserialize)]
#[table_name = "helm_charts"]
pub struct NewHelmChart {
    pub created_at: DateTime<Utc>,
    pub helm_registry_id: Uuid,
    pub image_digest: String,
    pub image_tag: String,
    pub values_ui: Option<Json<UiSchema>>,
    pub actions_schema: Option<Json<ChartExtActions>>,
    pub features: Option<Json<ChartExtFeatures>>,
    pub resource_types: Option<Json<ChartExtResourceTypes>>,
    pub error: Option<String>,
}

impl NewHelmChart {
    pub async fn insert(self) -> DbResult<HelmChart> {
        Ok(diesel::insert_into(helm_charts::table)
            .values(self)
            .get_result_async(pool())
            .await?)
    }
}

#[derive(AsChangeset)]
#[table_name = "helm_charts"]
pub struct UpdateHelmChart {
    pub available: Option<bool>,
}

impl UpdateHelmChart {
    pub async fn save(self, id: Uuid) -> DbResult<HelmChart> {
        Ok(diesel::update(helm_charts::table.find(id))
            .set(self)
            .get_result_async(pool())
            .await?)
    }
}
