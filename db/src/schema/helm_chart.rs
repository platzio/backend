use crate::{pool, DbError, DbResult, Paginated, DEFAULT_PAGE_SIZE};
use async_diesel::*;
use chrono::prelude::*;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::query_builder::{AstPass, Query, QueryFragment};
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
        tag_format_id -> Nullable<Uuid>,
        parsed_version -> Nullable<Varchar>,
        parsed_revision -> Nullable<Varchar>,
        parsed_branch -> Nullable<Varchar>,
        parsed_commit -> Nullable<Varchar>,
    }
}

#[derive(Debug, Identifiable, Queryable, QueryableByName, Serialize, DieselFilter)]
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
    pub tag_format_id: Option<Uuid>,
    pub parsed_version: Option<String>,
    pub parsed_revision: Option<String>,
    #[filter]
    pub parsed_branch: Option<String>,
    pub parsed_commit: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HelmChartExtraFilters {
    in_use: Option<bool>,
}

#[derive(QueryId)]
struct InUseHelmCharts<T>(T);

impl<T> Query for InUseHelmCharts<T>
where
    T: Query,
{
    type SqlType = T::SqlType;
}

impl<T> RunQueryDsl<PgConnection> for InUseHelmCharts<T> {}

impl<T> QueryFragment<Pg> for InUseHelmCharts<T>
where
    T: QueryFragment<Pg>,
{
    fn walk_ast(&self, mut pass: AstPass<Pg>) -> QueryResult<()> {
        pass.push_sql(
            "
            WITH first_used AS (
                SELECT
                    chart.helm_registry_id AS helm_registry_id,
                    chart.parsed_branch AS parsed_branch,
                    min(chart.created_at) AS created_at
                FROM
                    helm_charts chart
                INNER JOIN
                    deployments depl
                ON
                    chart.id = depl.helm_chart_id
                GROUP BY
                    chart.helm_registry_id,
                    chart.parsed_branch
            )
            SELECT \"helm_charts\".* FROM (",
        );
        self.0.walk_ast(pass.reborrow())?;
        pass.push_sql(
            ") AS helm_charts
            INNER JOIN
                first_used
            ON
                first_used.helm_registry_id = helm_charts.helm_registry_id AND
                first_used.parsed_branch = helm_charts.parsed_branch
            WHERE
                first_used.created_at <= helm_charts.created_at
            ",
        );
        Ok(())
    }
}

impl HelmChart {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(helm_charts::table.get_results_async(pool()).await?)
    }

    pub async fn all_filtered(
        filters: HelmChartFilters,
        extra_filters: HelmChartExtraFilters,
    ) -> DbResult<Paginated<Self>> {
        log::debug!("{:?} {:?}", filters, extra_filters);
        let conn = pool().get()?;
        let page = filters.page.unwrap_or(1);
        let per_page = filters.per_page.unwrap_or(DEFAULT_PAGE_SIZE);

        let (items, num_total) = tokio::task::spawn_blocking(move || {
            if extra_filters.in_use.unwrap_or_default() {
                InUseHelmCharts(Self::filter(&filters))
                    .paginate(Some(page))
                    .per_page(Some(per_page))
                    .load_and_count::<Self>(&conn)
            } else {
                Self::filter(&filters)
                    .paginate(Some(page))
                    .per_page(Some(per_page))
                    .load_and_count::<Self>(&conn)
            }
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
    pub tag_format_id: Option<Uuid>,
    pub parsed_version: Option<String>,
    pub parsed_revision: Option<String>,
    pub parsed_branch: Option<String>,
    pub parsed_commit: Option<String>,
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

#[derive(Debug, Default, AsChangeset)]
#[table_name = "helm_charts"]
pub struct HelmChartTagInfo {
    pub tag_format_id: Option<Uuid>,
    pub parsed_version: Option<String>,
    pub parsed_revision: Option<String>,
    pub parsed_branch: Option<String>,
    pub parsed_commit: Option<String>,
}

impl HelmChartTagInfo {
    pub async fn save(self, id: Uuid) -> DbResult<HelmChart> {
        Ok(diesel::update(helm_charts::table.find(id))
            .set(self)
            .get_result_async(pool())
            .await?)
    }
}
