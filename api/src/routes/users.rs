use crate::auth::CurUser;
use crate::permissions::verify_site_admin;
use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use platz_db::{UpdateUser, User};
use serde_json::json;
use uuid::Uuid;

#[actix_web::get("")]
async fn get_all() -> ApiResult {
    Ok(HttpResponse::Ok().json(User::all().await?))
}

#[actix_web::get("{id}")]
async fn get(id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(User::find(id.into_inner()).await?))
}

#[actix_web::put("/{id}")]
async fn update(
    cur_user: CurUser,
    id: web::Path<Uuid>,
    update: web::Json<UpdateUser>,
) -> ApiResult {
    verify_site_admin(cur_user.user().id).await?;
    let id = id.into_inner();
    if cur_user.user().id == id {
        Ok(HttpResponse::Forbidden().json(json!({
            "message": "You can't update your own user"
        })))
    } else {
        Ok(HttpResponse::Ok().json(update.into_inner().save(id).await?))
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/users")
            .service(get_all)
            .service(get)
            .service(update),
    );
}
