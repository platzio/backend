use crate::permissions::verify_env_admin;
use crate::result::ApiResult;
use actix_web::{delete, get, post, web, HttpResponse};
use platz_auth::ApiIdentity;
use platz_db::{
    DeploymentPermission, DeploymentPermissionFilters, NewDeploymentPermission, Paginated,
    UserDeploymentRole,
};
use uuid::Uuid;

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Deployment Permissions",
    operation_id = "allDeploymentPermissions",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    params(DeploymentPermissionFilters),
    responses(
        (
            status = OK,
            body = inline(Paginated<DeploymentPermission>),
        ),
    ),
)]
#[get("/deployment-permissions")]
async fn get_all(
    _identity: ApiIdentity,
    filters: web::Query<DeploymentPermissionFilters>,
) -> ApiResult {
    Ok(HttpResponse::Ok().json(DeploymentPermission::all_filtered(filters.into_inner()).await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Deployment Permissions",
    operation_id = "getDeploymentPermission",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    responses(
        (
            status = OK,
            body = DeploymentPermission,
        ),
    ),
)]
#[get("/deployment-permissions/{id}")]
async fn get_one(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(DeploymentPermission::find(id.into_inner()).await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Deployment Permissions",
    operation_id = "createDeploymentPermission",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    request_body = NewDeploymentPermission,
    responses(
        (
            status = CREATED,
            body = DeploymentPermission,
        ),
    ),
)]
#[post("/deployment-permissions")]
async fn create(
    identity: ApiIdentity,
    new_permission: web::Json<NewDeploymentPermission>,
) -> ApiResult {
    let new_permission = new_permission.into_inner();
    verify_env_admin(new_permission.env_id, &identity).await?;
    Ok(HttpResponse::Created().json(new_permission.insert().await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Deployment Permissions",
    operation_id = "deleteDeploymentPermission",
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
#[delete("/deployment-permissions/{id}")]
async fn delete(identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    let permission = DeploymentPermission::find(id.into_inner()).await?;
    verify_env_admin(permission.env_id, &identity).await?;
    permission.delete().await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(utoipa::OpenApi)]
#[openapi(
    tags((
        name = "Deployment Permissions",
        description = "\
APIs for setting deployment permissions per user.

See UserDeploymentRole for more information.
        ",
    )),
    paths(get_all, get_one, create, delete),
    components(schemas(DeploymentPermission, UserDeploymentRole, NewDeploymentPermission,))
)]
pub(super) struct OpenApi;
