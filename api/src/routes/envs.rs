use crate::auth::CurUser;
use crate::permissions::verify_site_admin;
use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use itertools::Itertools;
use platz_db::{Deployment, Env, EnvUserRole, NewEnv, NewEnvUserPermission, UpdateEnv};
use uuid::Uuid;

#[actix_web::get("")]
async fn get_all() -> ApiResult {
    Ok(HttpResponse::Ok().json(Env::all().await?))
}

#[actix_web::get("{id}")]
async fn get(id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(Env::find(id.into_inner()).await?))
}

#[actix_web::post("")]
async fn create(cur_user: CurUser, new_env: web::Json<NewEnv>) -> ApiResult {
    verify_site_admin(cur_user.user().id).await?;
    let env = new_env.into_inner().save().await?;
    NewEnvUserPermission {
        env_id: env.id,
        user_id: cur_user.user().id,
        role: EnvUserRole::Admin,
    }
    .insert()
    .await?;
    Ok(HttpResponse::Created().json(env))
}

#[actix_web::put("/{id}")]
async fn update(cur_user: CurUser, id: web::Path<Uuid>, update: web::Json<UpdateEnv>) -> ApiResult {
    let id = id.into_inner();
    verify_site_admin(cur_user.user().id).await?;

    if update.node_selector.is_some() || update.tolerations.is_some() {
        let reason = format!(
            "Env {} updated",
            [
                update.node_selector.as_ref().map(|_| "node selector"),
                update.tolerations.as_ref().map(|_| "tolerations"),
            ]
            .into_iter()
            .flatten()
            .join(", ")
        );
        Deployment::reinstall_all_for_env(id, cur_user.user(), reason).await?;
    }

    Ok(HttpResponse::Ok().json(update.into_inner().save(id).await?))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/envs")
            .service(get_all)
            .service(get)
            .service(create)
            .service(update),
    );
}
