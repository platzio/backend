use crate::auth::CurUser;
use crate::permissions::verify_env_admin;
use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use platz_db::{DeploymentPermission, NewDeploymentPermission};
use uuid::Uuid;

async fn get_all(_cur_user: CurUser) -> ApiResult {
    Ok(HttpResponse::Ok().json(DeploymentPermission::all().await?))
}

async fn get(_cur_user: CurUser, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(DeploymentPermission::find(id.into_inner()).await?))
}

async fn create(
    cur_user: CurUser,
    new_permission: web::Json<NewDeploymentPermission>,
) -> ApiResult {
    let new_permission = new_permission.into_inner();
    verify_env_admin(new_permission.env_id, cur_user.user().id).await?;
    Ok(HttpResponse::Created().json(new_permission.insert().await?))
}

async fn delete(cur_user: CurUser, id: web::Path<Uuid>) -> ApiResult {
    let permission = DeploymentPermission::find(id.into_inner()).await?;
    verify_env_admin(permission.env_id, cur_user.user().id).await?;
    permission.delete().await?;
    Ok(HttpResponse::NoContent().finish())
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(get_all));
    cfg.route("/{id}", web::get().to(get));
    cfg.route("", web::post().to(create));
    cfg.route("/{id}", web::delete().to(delete));
}
