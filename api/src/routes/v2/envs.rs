use super::deployments::using_error;
use crate::permissions::verify_site_admin;
use crate::result::ApiResult;
use actix_web::{delete, get, post, put, web, HttpResponse};
use itertools::Itertools;
use platz_auth::ApiIdentity;
use platz_db::{
    Deployment, Env, EnvFilters, EnvUserRole, NewEnv, NewEnvUserPermission, Paginated, UpdateEnv,
};
use serde_json::json;
use uuid::Uuid;

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Envs",
    operation_id = "allEnvs",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    params(EnvFilters),
    responses(
        (
            status = OK,
            body = inline(Paginated<Env>),
        ),
    ),
)]
#[get("/envs")]
async fn get_all(_identity: ApiIdentity, filters: web::Query<EnvFilters>) -> ApiResult {
    Ok(HttpResponse::Ok().json(Env::all_filtered(filters.into_inner()).await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Envs",
    operation_id = "getEnv",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    responses(
        (
            status = OK,
            body = Env,
        ),
    ),
)]
#[get("/envs/{id}")]
async fn get_one(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(Env::find(id.into_inner()).await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Envs",
    operation_id = "createEnv",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    request_body = NewEnv,
    responses(
        (
            status = CREATED,
            body = Env,
        ),
    ),
)]
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

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Envs",
    operation_id = "deleteEnv",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    responses(
        (
            status = NO_CONTENT,
        ),
    ),
)]
#[delete("/envs/{id}")]
async fn delete(identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    verify_site_admin(&identity).await?;
    let env = Env::find(id.into_inner()).await?;

    let dependents = Deployment::find_by_env_id(env.id).await?;
    if !dependents.is_empty() {
        return Ok(HttpResponse::Conflict().json(json!({
            "message": using_error("This environment can't be deleted because it has active deployments", dependents),
        })));
    }

    env.delete().await?;
    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Envs",
    operation_id = "updateEnv",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    request_body = UpdateEnv,
    responses(
        (
            status = OK,
            body = Env,
        ),
    ),
)]
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

#[derive(utoipa::OpenApi)]
#[openapi(
    tags((
        name = "Envs",
        description = "\
Envs contain deployments and all related settings resources for those
deployments, such as deployment permissions.
        ",
    )),
    paths(get_all, get_one, create, update, delete),
    components(schemas(
        Env,
        NewEnv,
        UpdateEnv,
    )),
)]
pub(super) struct OpenApi;
