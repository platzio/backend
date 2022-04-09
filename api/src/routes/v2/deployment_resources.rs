use crate::auth::CurIdentity;
use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use futures::future::try_join_all;
use platz_db::{
    Deployment, DeploymentResource, DeploymentResourceFilters, DeploymentResourceType,
    NewDeploymentResource, SyncStatus, UpdateDeploymentResource,
    UpdateDeploymentResourceSyncStatus,
};
use serde_json::json;
use uuid::Uuid;

async fn get_all(
    _cur_identity: CurIdentity,
    filters: web::Query<DeploymentResourceFilters>,
) -> ApiResult {
    let mut result = DeploymentResource::all_filtered(filters.into_inner()).await?;
    result.items = try_join_all(
        result
            .items
            .into_iter()
            .map(|resource| resource.without_sensitive_props()),
    )
    .await?;
    Ok(HttpResponse::Ok().json(result))
}

async fn get(_cur_identity: CurIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(
        DeploymentResource::find(id.into_inner())
            .await?
            .without_sensitive_props()
            .await?,
    ))
}

async fn create(
    _cur_identity: CurIdentity,
    new_resource: web::Json<NewDeploymentResource>,
) -> ApiResult {
    let new_resource = new_resource.into_inner();
    // TODO: Check allowed_role
    let resource = new_resource.insert().await?;
    Ok(HttpResponse::Created().json(resource))
}

async fn update(
    cur_identity: CurIdentity,
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
        sync_status: Some(SyncStatus::Updating),
        sync_reason: Some(None),
    }
    .save(new_resource.id)
    .await?;

    if let Some(reason) = reason {
        Deployment::reinstall_all_using(
            &resource_type.as_db_collection(),
            id,
            cur_identity.user(),
            reason.clone(),
        )
        .await?;
        Deployment::reinstall_all_using(
            &resource_type.as_legacy_db_collection(),
            id,
            cur_identity.user(),
            reason,
        )
        .await?;
    }

    Ok(HttpResponse::Ok().json(new_resource))
}

async fn delete(_cur_identity: CurIdentity, id: web::Path<Uuid>) -> ApiResult {
    let resource = DeploymentResource::find(id.into_inner()).await?;
    if !resource.exists {
        return Ok(HttpResponse::Conflict().json(json!({
            "message": "Trying to delete an already delete resource"
        })));
    }

    // TODO: Check allowed_role

    UpdateDeploymentResourceSyncStatus {
        sync_status: Some(SyncStatus::Deleting),
        sync_reason: Some(None),
    }
    .save(resource.id)
    .await?;

    Ok(HttpResponse::NoContent().finish())
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(get_all));
    cfg.route("/{id}", web::get().to(get));
    cfg.route("", web::post().to(create));
    cfg.route("/{id}", web::put().to(update));
    cfg.route("/{id}", web::delete().to(delete));
}
