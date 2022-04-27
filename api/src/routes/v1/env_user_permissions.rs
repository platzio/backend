use crate::permissions::verify_env_admin;
use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use platz_auth::ApiIdentity;
use platz_db::{EnvUserPermission, NewEnvUserPermission};
use serde_json::json;
use uuid::Uuid;

async fn get_all() -> ApiResult {
    Ok(HttpResponse::Ok().json(EnvUserPermission::all().await?))
}

async fn get(id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(EnvUserPermission::find(id.into_inner()).await?))
}

async fn create(
    identity: ApiIdentity,
    new_permission: web::Json<NewEnvUserPermission>,
) -> ApiResult {
    let new_permission = new_permission.into_inner();
    verify_env_admin(new_permission.env_id, &identity).await?;
    Ok(HttpResponse::Created().json(new_permission.insert().await?))
}

async fn delete(identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    let permission = EnvUserPermission::find(id.into_inner()).await?;
    verify_env_admin(permission.env_id, &identity).await?;
    if Some(permission.user_id) == identity.inner().user_id() {
        return Ok(HttpResponse::Conflict().json(json!({
            "message": "You can't delete your own permissions"
        })));
    }
    permission.delete().await?;
    Ok(HttpResponse::NoContent().finish())
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(get_all));
    cfg.route("/{id}", web::get().to(get));
    cfg.route("", web::post().to(create));
    cfg.route("/{id}", web::delete().to(delete));
}
