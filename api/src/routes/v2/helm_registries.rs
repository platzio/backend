use crate::auth::CurUser;
use crate::permissions::verify_site_admin;
use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use platz_db::{HelmRegistry, UpdateHelmRegistry};
use uuid::Uuid;

async fn get_all(_cur_user: CurUser) -> ApiResult {
    Ok(HttpResponse::Ok().json(HelmRegistry::all().await?))
}

async fn get(_cur_user: CurUser, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(HelmRegistry::find(id.into_inner()).await?))
}

async fn update(
    cur_user: CurUser,
    id: web::Path<Uuid>,
    data: web::Json<UpdateHelmRegistry>,
) -> ApiResult {
    verify_site_admin(cur_user.user().id).await?;
    let id = id.into_inner();
    let data = data.into_inner();
    Ok(HttpResponse::Ok().json(data.save(id).await?))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(get_all));
    cfg.route("/{id}", web::get().to(get));
    cfg.route("/{id}", web::put().to(update));
}
