use crate::auth::CurIdentity;
use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use platz_db::{HelmChart, HelmChartFilters};
use uuid::Uuid;

async fn get_all(_cur_identity: CurIdentity, filters: web::Query<HelmChartFilters>) -> ApiResult {
    Ok(HttpResponse::Ok().json(HelmChart::all_filtered(filters.into_inner()).await?))
}

async fn get(_cur_identity: CurIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(HelmChart::find(id.into_inner()).await?))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(get_all));
    cfg.route("/{id}", web::get().to(get));
}
