use crate::result::ApiResult;
use actix_web::{HttpResponse, delete, get, post, put, web};
use futures::future::try_join_all;
use platz_auth::ApiIdentity;
use platz_db::{
    diesel_pagination::{Paginated, PaginationParams},
    schema::{
        deployment::Deployment,
        deployment_resource::{
            DeploymentResource, DeploymentResourceFilters, DeploymentResourceSyncStatus,
            NewDeploymentResource, UpdateDeploymentResource, UpdateDeploymentResourceSyncStatus,
        },
        deployment_resource_type::DeploymentResourceType,
    },
};
use serde_json::json;
use uuid::Uuid;

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Deployment Resources",
    operation_id = "allDeploymentResources",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    params(DeploymentResourceFilters),
    responses(
        (
            status = OK,
            body = Paginated<DeploymentResource>,
        ),
    ),
)]
#[get("/deployment-resources")]
async fn get_all(
    _identity: ApiIdentity,
    filters: web::Query<DeploymentResourceFilters>,
    pagination: web::Query<PaginationParams>,
) -> ApiResult {
    let mut result =
        DeploymentResource::all_filtered(filters.into_inner(), pagination.into_inner()).await?;
    result.items = try_join_all(
        result
            .items
            .into_iter()
            .map(|resource| resource.without_sensitive_props()),
    )
    .await?;
    Ok(HttpResponse::Ok().json(result))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Deployment Resources",
    operation_id = "getDeploymentResource",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    responses(
        (
            status = OK,
            body = DeploymentResource,
        ),
    ),
)]
#[get("/deployment-resources/{id}")]
async fn get_one(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(
        DeploymentResource::find(id.into_inner())
            .await?
            .without_sensitive_props()
            .await?,
    ))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Deployment Resources",
    operation_id = "createDeploymentResource",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    request_body = NewDeploymentResource,
    responses(
        (
            status = CREATED,
            body = DeploymentResource,
        ),
    ),
)]
#[post("/deployment-resources")]
async fn create(
    _identity: ApiIdentity,
    new_resource: web::Json<NewDeploymentResource>,
) -> ApiResult {
    let new_resource = new_resource.into_inner();
    // TODO: Check allowed_role
    let resource = new_resource.insert().await?;
    Ok(HttpResponse::Created().json(resource))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Deployment Resources",
    operation_id = "updateDeploymentResource",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    request_body = UpdateDeploymentResource,
    responses(
        (
            status = OK,
            body = DeploymentResource,
        ),
    ),
)]
#[put("/deployment-resources/{id}")]
async fn update(
    identity: ApiIdentity,
    id: web::Path<Uuid>,
    update: web::Json<UpdateDeploymentResource>,
) -> ApiResult {
    let id = id.into_inner();
    let update = update.into_inner();
    let old_resource = DeploymentResource::find(id).await?;
    let resource_type = DeploymentResourceType::find(old_resource.type_id).await?;
    let resource_spec = resource_type.spec()?;

    let reason = match (update.name.as_ref(), update.props.as_ref()) {
        (None, None) => None,
        (Some(new_name), None) => Some(format!(
            "{} {} renamed to {}",
            resource_spec.name_singular, old_resource.name, new_name,
        )),
        (None, Some(_)) => Some(format!(
            "{} {} updated",
            resource_spec.name_singular, old_resource.name
        )),
        (Some(new_name), Some(_)) => Some(format!(
            "{} {} updated and renamed to {}",
            resource_spec.name_singular, old_resource.name, new_name,
        )),
    };

    let new_resource = update.save(id).await?;

    UpdateDeploymentResourceSyncStatus {
        sync_status: DeploymentResourceSyncStatus::Updating,
        sync_reason: None,
    }
    .save(new_resource.id)
    .await?;

    if let Some(reason) = reason {
        Deployment::reinstall_all_using(
            &resource_type.as_db_collection().await,
            id,
            &identity,
            reason.clone(),
        )
        .await?;
        Deployment::reinstall_all_using(
            &resource_type.as_legacy_db_collection(),
            id,
            &identity,
            reason,
        )
        .await?;
    }

    Ok(HttpResponse::Ok().json(new_resource))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Deployment Resources",
    operation_id = "deleteDeploymentResource",
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
#[delete("/deployment-resources/{id}")]
async fn delete(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    let resource = DeploymentResource::find(id.into_inner()).await?;
    if !resource.exists {
        return Ok(HttpResponse::Conflict().json(json!({
            "message": "Trying to delete an already delete resource"
        })));
    }

    // TODO: Check allowed_role

    UpdateDeploymentResourceSyncStatus {
        sync_status: DeploymentResourceSyncStatus::Deleting,
        sync_reason: None,
    }
    .save(resource.id)
    .await?;

    Ok(HttpResponse::NoContent().finish())
}

#[derive(utoipa::OpenApi)]
#[openapi(
    tags((
        name = "Deployment Resources",
        description = "",
    )),
    paths(get_all, get_one, create, update, delete),
)]
pub(super) struct OpenApi;
