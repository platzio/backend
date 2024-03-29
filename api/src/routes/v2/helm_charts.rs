use crate::result::ApiResult;
use actix_web::{get, web, HttpResponse};
use platz_auth::ApiIdentity;
use platz_db::{HelmChart, HelmChartExtraFilters, HelmChartFilters, Paginated};
use uuid::Uuid;

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Helm Charts",
    operation_id = "allHelmCharts",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    params(HelmChartFilters),
    responses(
        (
            status = OK,
            body = inline(Paginated<HelmChart>),
        ),
    ),
)]
#[get("/helm-charts")]
async fn get_all(
    _identity: ApiIdentity,
    filters: web::Query<HelmChartFilters>,
    extra_filters: web::Query<HelmChartExtraFilters>,
) -> ApiResult {
    Ok(HttpResponse::Ok()
        .json(HelmChart::all_filtered(filters.into_inner(), extra_filters.into_inner()).await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Helm Charts",
    operation_id = "getHelmChart",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    responses(
        (
            status = OK,
            body = HelmChart,
        ),
    ),
)]
#[get("/helm-charts/{id}")]
async fn get_one(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(HelmChart::find(id.into_inner()).await?))
}

#[derive(utoipa::OpenApi)]
#[openapi(
    tags((
        name = "Helm Charts",
        description = "\
This collection contains Helm charts detected by the chart-discovery service.
        ",
    )),
    paths(get_all, get_one),
    components(schemas(
        HelmChart,
    )),
)]
pub(super) struct OpenApi;
