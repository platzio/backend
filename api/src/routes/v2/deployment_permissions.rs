use crate::auth::CurIdentity;
use crate::permissions::verify_env_admin;
use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use platz_db::{DeploymentPermission, DeploymentPermissionFilters, NewDeploymentPermission};
use uuid::Uuid;

async fn get_all(
    _cur_identity: CurIdentity,
    filters: web::Query<DeploymentPermissionFilters>,
) -> ApiResult {
    Ok(HttpResponse::Ok().json(DeploymentPermission::all_filtered(filters.into_inner()).await?))
}

async fn get(_cur_identity: CurIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(DeploymentPermission::find(id.into_inner()).await?))
}

async fn create(
    cur_identity: CurIdentity,
    new_permission: web::Json<NewDeploymentPermission>,
) -> ApiResult {
    let new_permission = new_permission.into_inner();
    verify_env_admin(new_permission.env_id, cur_identity.user().id).await?;
    Ok(HttpResponse::Created().json(new_permission.insert().await?))
}

async fn delete(cur_identity: CurIdentity, id: web::Path<Uuid>) -> ApiResult {
    let permission = DeploymentPermission::find(id.into_inner()).await?;
    verify_env_admin(permission.env_id, cur_identity.user().id).await?;
    permission.delete().await?;
    Ok(HttpResponse::NoContent().finish())
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(get_all));
    cfg.route("/{id}", web::get().to(get));
    cfg.route("", web::post().to(create));
    cfg.route("/{id}", web::delete().to(delete));
}
