use crate::permissions::verify_site_admin;
use crate::result::ApiResult;
use actix_web::{get, post, put, web, HttpResponse};
use itertools::Itertools;
use platz_auth::ApiIdentity;
use platz_db::{Deployment, Env, EnvFilters, EnvUserRole, NewEnv, NewEnvUserPermission, UpdateEnv};
use uuid::Uuid;

#[get("/envs")]
async fn get_all(_identity: ApiIdentity, filters: web::Query<EnvFilters>) -> ApiResult {
    Ok(HttpResponse::Ok().json(Env::all_filtered(filters.into_inner()).await?))
}

#[get("/envs/{id}")]
async fn get_one(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(Env::find(id.into_inner()).await?))
}

#[post("/envs")]
async fn create(identity: ApiIdentity, new_env: web::Json<NewEnv>) -> ApiResult {
    verify_site_admin(&identity).await?;
    let env = new_env.into_inner().save().await?;
    NewEnvUserPermission {
        env_id: env.id,
        user_id: identity
            .inner()
            .user_id()
            .expect("Site admin must be a user"),
        role: EnvUserRole::Admin,
    }
    .insert()
    .await?;
    Ok(HttpResponse::Created().json(env))
}

#[put("/envs/{id}")]
async fn update(
    identity: ApiIdentity,
    id: web::Path<Uuid>,
    update: web::Json<UpdateEnv>,
) -> ApiResult {
    let id = id.into_inner();
    verify_site_admin(&identity).await?;

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
        Deployment::reinstall_all_for_env(id, &identity, reason).await?;
    }

    Ok(HttpResponse::Ok().json(update.into_inner().save(id).await?))
}
