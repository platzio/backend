use crate::permissions::verify_env_admin;
use crate::result::ApiResult;
use actix_web::{delete, get, post, web, HttpResponse};
use platz_auth::ApiIdentity;
use platz_db::{EnvUserPermission, EnvUserPermissionFilters, NewEnvUserPermission};
use serde_json::json;
use uuid::Uuid;

#[get("/env-user-permissions")]
async fn get_all(
    _identity: ApiIdentity,
    filters: web::Query<EnvUserPermissionFilters>,
) -> ApiResult {
    Ok(HttpResponse::Ok().json(EnvUserPermission::all_filtered(filters.into_inner()).await?))
}

#[get("/env-user-permissions/{id}")]
async fn get_one(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(EnvUserPermission::find(id.into_inner()).await?))
}

#[post("/env-user-permissions")]
async fn create(
    identity: ApiIdentity,
    new_permission: web::Json<NewEnvUserPermission>,
) -> ApiResult {
    let new_permission = new_permission.into_inner();
    verify_env_admin(new_permission.env_id, &identity).await?;
    Ok(HttpResponse::Created().json(new_permission.insert().await?))
}

#[delete("/env-user-permissions/{id}")]
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
