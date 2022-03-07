use crate::auth::CurUser;
use crate::permissions::verify_env_admin;
use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use platz_db::{EnvUserPermission, NewEnvUserPermission};
use serde_json::json;
use uuid::Uuid;

#[actix_web::get("")]
async fn get_all() -> ApiResult {
    Ok(HttpResponse::Ok().json(EnvUserPermission::all().await?))
}

#[actix_web::get("/{id}")]
async fn get(id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(EnvUserPermission::find(id.into_inner()).await?))
}

#[actix_web::post("")]
async fn create(cur_user: CurUser, new_permission: web::Json<NewEnvUserPermission>) -> ApiResult {
    let new_permission = new_permission.into_inner();
    verify_env_admin(new_permission.env_id, cur_user.user().id).await?;
    Ok(HttpResponse::Created().json(new_permission.insert().await?))
}

#[actix_web::delete("/{id}")]
async fn delete(cur_user: CurUser, id: web::Path<Uuid>) -> ApiResult {
    let permission = EnvUserPermission::find(id.into_inner()).await?;
    verify_env_admin(permission.env_id, cur_user.user().id).await?;
    if permission.user_id == cur_user.user().id {
        return Ok(HttpResponse::Conflict().json(json!({
            "message": "You can't delete your own permissions"
        })));
    }
    permission.delete().await?;
    Ok(HttpResponse::NoContent().finish())
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/env-user-permissions")
            .service(get_all)
            .service(get)
            .service(create)
            .service(delete),
    );
}
