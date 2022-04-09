use crate::{pool, DbError, DbResult, Paginated, DEFAULT_PAGE_SIZE};
use async_diesel::*;
use chrono::prelude::*;
use diesel::prelude::*;
use diesel::QueryDsl;
use diesel_filter::{DieselFilter, Paginate};
pub use diesel_json::Json;
use platz_chart_ext::resource_types::ChartExtResourceTypes;
use platz_chart_ext::{ChartExtActions, ChartExtFeatures, UiSchema};
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

#[derive(Debug, Identifiable, Queryable, Serialize, DieselFilter)]
#[table_name = "helm_charts"]
#[pagination]
pub struct HelmChart {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    #[filter]
    pub helm_registry_id: Uuid,
    pub image_digest: String,
    pub image_tag: String,
    pub available: bool,
    pub values_ui: Option<serde_json::Value>,
    pub actions_schema: Option<serde_json::Value>,
    pub features: Option<serde_json::Value>,
    pub resource_types: Option<serde_json::Value>,
    pub error: Option<String>,
}

impl HelmChart {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(helm_charts::table.get_results_async(pool()).await?)
    }

    pub async fn all_filtered(filters: HelmChartFilters) -> DbResult<Paginated<Self>> {
        let conn = pool().get()?;
        let page = filters.page.unwrap_or(1);
        let per_page = filters.per_page.unwrap_or(DEFAULT_PAGE_SIZE);
        let (items, num_total) = tokio::task::spawn_blocking(move || {
            Self::filter(&filters)
                .paginate(Some(page))
                .per_page(Some(per_page))
                .load_and_count::<Self>(&conn)
        })
        .await
        .unwrap()?;
        Ok(Paginated {
            page,
            per_page,
            num_total,
            items,
        })
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

    pub fn actions_schema(&self) -> DbResult<ChartExtActions> {
        match self.actions_schema.as_ref() {
            Some(value) => match serde_json::from_value::<ChartExtActions>(value.clone()) {
                Ok(schema) => Ok(schema),
                Err(err) => Err(DbError::HelmChartActionsSchemaParseError(err)),
            },
            None => Err(DbError::HelmChartNoActionsSchema),
        }
    }

    pub fn features(&self) -> DbResult<ChartExtFeatures> {
        Ok(match self.features.as_ref() {
            None => Default::default(),
            Some(value) => serde_json::from_value(value.clone())
                .map_err(DbError::HelmChartFeaturesParsingError)?,
        })
    }

    pub fn resource_types(&self) -> DbResult<ChartExtResourceTypes> {
        Ok(match self.resource_types.as_ref() {
            None => Default::default(),
            Some(value) => serde_json::from_value(value.clone())
                .map_err(DbError::HelmChartResourceTypesParseError)?,
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
