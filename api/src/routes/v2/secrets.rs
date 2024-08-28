use super::deployments::using_error;
use crate::permissions::verify_env_admin;
use crate::result::ApiResult;
use actix_web::{delete, get, post, put, web, HttpResponse};
use platz_auth::ApiIdentity;
use platz_db::{
    DbTable, DbTableOrDeploymentResource, Deployment, NewSecret, Paginated, Secret, SecretFilters,
    UpdateSecret,
};
use serde_json::json;
use uuid::Uuid;

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Secrets",
    operation_id = "allSecrets",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    params(SecretFilters),
    responses(
        (
            status = OK,
            body = inline(Paginated<Secret>),
        ),
    ),
)]
#[get("/secrets")]
async fn get_all(_identity: ApiIdentity, filters: web::Query<SecretFilters>) -> ApiResult {
    Ok(HttpResponse::Ok().json(Secret::all_filtered(filters.into_inner()).await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Secrets",
    operation_id = "getSecret",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    responses(
        (
            status = OK,
            body = Secret,
        ),
    ),
)]
#[get("/secrets/{id}")]
async fn get_one(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(Secret::find(id.into_inner()).await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Secrets",
    operation_id = "createSecret",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    request_body = NewSecret,
    responses(
        (
            status = CREATED,
            body = Secret,
        ),
    ),
)]
#[post("/secrets")]
async fn create(identity: ApiIdentity, new_secret: web::Json<NewSecret>) -> ApiResult {
    let new_secret = new_secret.into_inner();
    verify_env_admin(new_secret.env_id, &identity).await?;
    Ok(HttpResponse::Created().json(new_secret.insert().await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Secrets",
    operation_id = "updateSecret",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    request_body = UpdateSecret,
    responses(
        (
            status = OK,
            body = Secret,
        ),
    ),
)]
#[put("/secrets/{id}")]
async fn update(
    identity: ApiIdentity,
    id: web::Path<Uuid>,
    update: web::Json<UpdateSecret>,
) -> ApiResult {
    let id = id.into_inner();
    let update = update.into_inner();

    let old = Secret::find(id).await?;
    verify_env_admin(old.env_id, &identity).await?;
    let new = update.save(id).await?;

    Deployment::reinstall_all_using(
        &DbTableOrDeploymentResource::DbTable(DbTable::Secrets),
        id,
        &identity,
        format!("{} secret has been updated", new.collection),
    )
    .await?;

    Ok(HttpResponse::Ok().json(new))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Secrets",
    operation_id = "deleteSecret",
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
#[delete("/secrets/{id}")]
async fn delete(identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    let id = id.into_inner();
    let secret = Secret::find(id).await?;

    verify_env_admin(secret.env_id, &identity).await?;

    let dependents =
        Deployment::find_using(&DbTableOrDeploymentResource::DbTable(DbTable::Secrets), id).await?;
    if !dependents.is_empty() {
        return Ok(HttpResponse::Conflict().json(json!({
            "message": using_error("This deployment can't be deleted because other deployments depend on it", dependents),
        })));
    }

    secret.delete().await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(utoipa::OpenApi)]
#[openapi(
    tags((
        name = "Secrets",
        description = "\
Secrets are stored in envs and can be referenced by chart inputs by using the
`secrets` collection.

Kubernetes secrets are created during deployment as defined in the chart
extensions. See chart extensions documentation for more information.
        ",
    )),
    paths(get_all, get_one, create, update, delete),
    components(schemas(
        Secret,
        NewSecret,
        UpdateSecret,
    )),
)]
pub(super) struct OpenApi;
