use crate::auth::CurUser;
use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use platz_db::HelmChart;
use uuid::Uuid;

async fn get_all(_cur_user: CurUser) -> ApiResult {
    Ok(HttpResponse::Ok().json(HelmChart::all().await?))
}

async fn get(_cur_user: CurUser, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(HelmChart::find(id.into_inner()).await?))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(get_all));
    cfg.route("/{id}", web::get().to(get));
}
