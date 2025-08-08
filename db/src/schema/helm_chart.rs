use super::helm_registry::helm_registries;
use crate::{db_conn, DbError, DbResult};
use chrono::prelude::*;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::query_builder::{AstPass, Query, QueryFragment};
use diesel_async::RunQueryDsl;
use diesel_filter::DieselFilter;
use diesel_json::Json;
use diesel_pagination::{Paginate, Paginated, PaginationParams};
use platz_chart_ext::{
    resource_types::ChartExtResourceTypes, ChartExtActions, ChartExtFeatures, UiSchema,
};
use serde::{Deserialize, Serialize};
use std::ops::DerefMut;
use tracing::debug;
use utoipa::ToSchema;
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

diesel::joinable!(helm_charts -> helm_registries(helm_registry_id));

diesel::allow_tables_to_appear_in_same_query!(helm_charts, helm_registries);

#[derive(Debug, Identifiable, Queryable, QueryableByName, Serialize, DieselFilter, ToSchema)]
#[diesel(table_name = helm_charts)]
pub struct HelmChart {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    #[filter]
    pub helm_registry_id: Uuid,
    pub image_digest: String,
    pub image_tag: String,
    pub available: bool,
    #[schema(required, value_type = Option<UiSchema>)]
    pub values_ui: Option<serde_json::Value>,
    #[schema(required, value_type = Option<ChartExtActions>)]
    pub actions_schema: Option<serde_json::Value>,
    #[schema(required, value_type = Option<ChartExtFeatures>)]
    pub features: Option<serde_json::Value>,
    #[schema(required, value_type = Option<ChartExtResourceTypes>)]
    pub resource_types: Option<serde_json::Value>,
    #[schema(required)]
    pub error: Option<String>,
    #[schema(required)]
    pub tag_format_id: Option<Uuid>,
    #[schema(required)]
    pub parsed_version: Option<String>,
    #[schema(required)]
    pub parsed_revision: Option<String>,
    #[filter]
    #[schema(required)]
    pub parsed_branch: Option<String>,
    #[schema(required)]
    pub parsed_commit: Option<String>,
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct HelmChartExtraFilters {
    #[schema(required)]
    in_use: Option<bool>,
    #[schema(required)]
    kind_id: Option<Uuid>,
}

#[derive(diesel::QueryId)]
struct InUseHelmCharts<T>(T);

impl<T> Query for InUseHelmCharts<T>
where
    T: Query,
{
    type SqlType = T::SqlType;
}

impl<T> QueryFragment<Pg> for InUseHelmCharts<T>
where
    T: QueryFragment<Pg>,
{
    fn walk_ast<'a>(&'a self, mut pass: AstPass<'_, 'a, Pg>) -> QueryResult<()> {
        pass.push_sql(
            r#"WITH charts_in_use AS (
  SELECT
    *
  FROM
    helm_charts
  WHERE
    EXISTS (
      SELECT
        *
      FROM
        deployments
      WHERE
        deployments.helm_chart_id = helm_charts.id)
),
latest_per_branch AS (
  SELECT
    id,
    helm_registry_id,
    image_tag,
    parsed_branch,
    FIRST_VALUE(id) OVER (PARTITION BY (helm_registry_id,
      parsed_branch) ORDER BY created_at DESC) AS latest_available_id
FROM
  helm_charts
  WHERE
    EXISTS (
      SELECT
        *
      FROM
        charts_in_use
      WHERE
        charts_in_use.helm_registry_id = helm_charts.helm_registry_id
        AND charts_in_use.parsed_branch = helm_charts.parsed_branch))
SELECT
  *
FROM ("#,
        );
        self.0.walk_ast(pass.reborrow())?;
        pass.push_sql(
            ") AS helm_charts

           WHERE (id IN (
    SELECT
      latest_available_id
    FROM
      latest_per_branch)
    OR EXISTS (
      SELECT
        *
      FROM
        deployments
      WHERE
        deployments.helm_chart_id = helm_charts.id))
            ",
        );
        Ok(())
    }
}

impl HelmChart {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(helm_charts::table
            .get_results(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn all_filtered(
        filters: HelmChartFilters,
        extra_filters: HelmChartExtraFilters,
        pagination: PaginationParams,
    ) -> DbResult<Paginated<Self>> {
        debug!("{:?} {:?}", filters, extra_filters);

        Ok(match (
            extra_filters.in_use.unwrap_or_default(),
            extra_filters.kind_id,
        ) {
            (true, Some(kind_id)) => {
                InUseHelmCharts(
                    Self::filter(filters)
                        .order_by(helm_charts::created_at.desc())
                        .inner_join(helm_registries::table)
                        .filter(helm_registries::kind_id.eq(kind_id))
                        .select(helm_charts::all_columns),
                )
                .paginate(pagination)
                .load_and_count(db_conn().await?.deref_mut())
                .await
            }
            (true, None) => {
                InUseHelmCharts(Self::filter(filters).order_by(helm_charts::created_at.desc()))
                    .paginate(pagination)
                    .load_and_count(db_conn().await?.deref_mut())
                    .await
            }
            (false, Some(kind_id)) => {
                Self::filter(filters)
                    .order_by(helm_charts::created_at.desc())
                    .inner_join(helm_registries::table)
                    .filter(helm_registries::kind_id.eq(kind_id))
                    .select(helm_charts::all_columns)
                    .paginate(pagination)
                    .load_and_count(db_conn().await?.deref_mut())
                    .await
            }
            (false, None) => {
                Self::filter(filters)
                    .order_by(helm_charts::created_at.desc())
                    .paginate(pagination)
                    .load_and_count(db_conn().await?.deref_mut())
                    .await
            }
        }?)
    }

    pub async fn find(id: Uuid) -> DbResult<Self> {
        Ok(helm_charts::table
            .find(id)
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn find_by_registry_and_digest(
        registry_id: Uuid,
        image_digest: String,
    ) -> DbResult<Option<Self>> {
        Ok(helm_charts::table
            .filter(helm_charts::helm_registry_id.eq(registry_id))
            .filter(helm_charts::image_digest.eq(image_digest))
            .first(db_conn().await?.deref_mut())
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
#[diesel(table_name = helm_charts)]
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
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }
}

#[derive(AsChangeset)]
#[diesel(table_name = helm_charts)]
pub struct UpdateHelmChart {
    pub available: bool,
}

impl UpdateHelmChart {
    pub async fn save(self, id: Uuid) -> DbResult<HelmChart> {
        Ok(diesel::update(helm_charts::table.find(id))
            .set(self)
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }
}

#[derive(Debug, Default, AsChangeset, ToSchema)]
#[diesel(table_name = helm_charts)]
pub struct HelmChartTagInfo {
    #[schema(required)]
    pub tag_format_id: Option<Uuid>,
    #[schema(required)]
    pub parsed_version: Option<String>,
    #[schema(required)]
    pub parsed_revision: Option<String>,
    #[schema(required)]
    pub parsed_branch: Option<String>,
    #[schema(required)]
    pub parsed_commit: Option<String>,
}

impl HelmChartTagInfo {
    pub async fn save(self, id: Uuid) -> DbResult<HelmChart> {
        Ok(diesel::update(helm_charts::table.find(id))
            .set(self)
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }
}
