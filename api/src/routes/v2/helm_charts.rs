use crate::result::ApiResult;
use actix_web::{get, web, HttpResponse};
use platz_auth::ApiIdentity;
use platz_db::{HelmChart, HelmChartExtraFilters, HelmChartFilters};
use uuid::Uuid;

#[get("/helm-charts")]
async fn get_all(
    _identity: ApiIdentity,
    filters: web::Query<HelmChartFilters>,
    extra_filters: web::Query<HelmChartExtraFilters>,
) -> ApiResult {
    Ok(HttpResponse::Ok()
        .json(HelmChart::all_filtered(filters.into_inner(), extra_filters.into_inner()).await?))
}

#[get("/helm-charts/{id}")]
async fn get_one(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(HelmChart::find(id.into_inner()).await?))
}
