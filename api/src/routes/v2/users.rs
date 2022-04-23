use crate::auth::ApiIdentity;
use crate::permissions::verify_site_admin;
use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use platz_db::{UpdateUser, User, UserFilters};
use serde_json::json;
use uuid::Uuid;

async fn get_all(_identity: ApiIdentity, filters: web::Query<UserFilters>) -> ApiResult {
    Ok(HttpResponse::Ok().json(User::all_filtered(filters.into_inner()).await?))
}

async fn get(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(User::find(id.into_inner()).await?))
}

async fn update(
    identity: ApiIdentity,
    id: web::Path<Uuid>,
    update: web::Json<UpdateUser>,
) -> ApiResult {
    verify_site_admin(&identity).await?;
    let id = id.into_inner();
    if identity.inner().user_id() == Some(id) {
        Ok(HttpResponse::Forbidden().json(json!({
            "message": "You can't update your own user"
        })))
    } else {
        Ok(HttpResponse::Ok().json(update.into_inner().save(id).await?))
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(get_all));
    cfg.route("/{id}", web::get().to(get));
    cfg.route("/{id}", web::put().to(update));
}
