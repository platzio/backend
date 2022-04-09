use crate::auth::CurIdentity;
use crate::permissions::verify_site_admin;
use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use platz_db::{HelmRegistry, HelmRegistryFilters, UpdateHelmRegistry};
use uuid::Uuid;

async fn get_all(
    _cur_identity: CurIdentity,
    filters: web::Query<HelmRegistryFilters>,
) -> ApiResult {
    Ok(HttpResponse::Ok().json(HelmRegistry::all_filtered(filters.into_inner()).await?))
}

async fn get(_cur_identity: CurIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(HelmRegistry::find(id.into_inner()).await?))
}

async fn update(
    cur_identity: CurIdentity,
    id: web::Path<Uuid>,
    data: web::Json<UpdateHelmRegistry>,
) -> ApiResult {
    verify_site_admin(cur_identity.user().id).await?;
    let id = id.into_inner();
    let data = data.into_inner();
    Ok(HttpResponse::Ok().json(data.save(id).await?))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(get_all));
    cfg.route("/{id}", web::get().to(get));
    cfg.route("/{id}", web::put().to(update));
}
