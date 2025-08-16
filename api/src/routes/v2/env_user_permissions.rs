use crate::{permissions::verify_env_admin, result::ApiResult};
use actix_web::{HttpResponse, delete, get, post, web};
use platz_auth::ApiIdentity;
use platz_db::{
    diesel_pagination::{Paginated, PaginationParams},
    schema::env_user_permission::{
        EnvUserPermission, EnvUserPermissionFilters, NewEnvUserPermission,
    },
};
use serde_json::json;
use uuid::Uuid;

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Env User Permissions",
    operation_id = "allEnvUserPermissions",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    params(EnvUserPermissionFilters),
    responses(
        (
            status = OK,
            body = Paginated<EnvUserPermission>,
        ),
    ),
)]
#[get("/env-user-permissions")]
async fn get_all(
    _identity: ApiIdentity,
    filters: web::Query<EnvUserPermissionFilters>,
    pagination: web::Query<PaginationParams>,
) -> ApiResult {
    Ok(HttpResponse::Ok().json(
        EnvUserPermission::all_filtered(filters.into_inner(), pagination.into_inner()).await?,
    ))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Env User Permissions",
    operation_id = "getEnvUserPermission",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    responses(
        (
            status = OK,
            body = EnvUserPermission,
        ),
    ),
)]
#[get("/env-user-permissions/{id}")]
async fn get_one(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(EnvUserPermission::find(id.into_inner()).await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Env User Permissions",
    operation_id = "createEnvUserPermission",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    request_body = NewEnvUserPermission,
    responses(
        (
            status = CREATED,
            body = EnvUserPermission,
        ),
    ),
)]
#[post("/env-user-permissions")]
async fn create(
    identity: ApiIdentity,
    new_permission: web::Json<NewEnvUserPermission>,
) -> ApiResult {
    let new_permission = new_permission.into_inner();
    verify_env_admin(new_permission.env_id, &identity).await?;
    Ok(HttpResponse::Created().json(new_permission.insert().await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Env User Permissions",
    operation_id = "deleteEnvUserPermission",
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

#[derive(utoipa::OpenApi)]
#[openapi(
    tags((
        name = "Env User Permissions",
        description = "\
Controls which envs each user can see.
        ",
    )),
    paths(get_all, get_one, create, delete),
)]
pub(super) struct OpenApi;
