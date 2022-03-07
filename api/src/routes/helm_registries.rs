use crate::auth::CurUser;
use crate::permissions::verify_site_admin;
use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use platz_db::{HelmRegistry, UpdateHelmRegistry};
use uuid::Uuid;

#[actix_web::get("")]
async fn get_all() -> ApiResult {
    Ok(HttpResponse::Ok().json(HelmRegistry::all().await?))
}

#[actix_web::get("/{id}")]
async fn get(id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(HelmRegistry::find(id.into_inner()).await?))
}

#[actix_web::put("/{id}")]
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
    cfg.service(
        web::scope("/api/v1/helm-registries")
            .service(get_all)
            .service(get)
            .service(update),
    );
}
