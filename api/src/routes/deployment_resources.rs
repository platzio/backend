use crate::auth::CurUser;
use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use futures::future::try_join_all;
use platz_db::{
    Deployment, DeploymentResource, DeploymentResourceType, NewDeploymentResource,
    UpdateDeploymentResource,
};
use uuid::Uuid;

#[actix_web::get("")]
async fn get_all() -> ApiResult {
    Ok(HttpResponse::Ok().json(
        try_join_all(
            DeploymentResource::all()
                .await?
                .into_iter()
                .map(|resource| resource.without_sensitive_props()),
        )
        .await?,
    ))
}

#[actix_web::get("/{id}")]
async fn get(id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(DeploymentResource::find(id.into_inner()).await?))
}

#[actix_web::post("")]
async fn create(_cur_user: CurUser, new_resource: web::Json<NewDeploymentResource>) -> ApiResult {
    let new_resource = new_resource.into_inner();
    let resource_type = DeploymentResourceType::find(new_resource.type_id).await?;
    let resource_spec = resource_type.spec()?;
    // TODO: Check allowed_role
    let resource = new_resource.insert().await?;
    if let Some(target) = resource_spec.lifecycle.create.target.as_ref() {
        resource.sync_to(target).await?;
    }
    Ok(HttpResponse::Created().json(resource))
}

#[actix_web::put("/{id}")]
async fn update(
    cur_user: CurUser,
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
    if let Some(target) = resource_spec.lifecycle.update.target.as_ref() {
        new_resource.sync_to(target).await?;
    }

    if let Some(reason) = reason {
        Deployment::reinstall_all_using(
            &resource_type.as_db_collection(),
            id,
            cur_user.user(),
            reason.clone(),
        )
        .await?;
        Deployment::reinstall_all_using(
            &resource_type.as_legacy_db_collection(),
            id,
            cur_user.user(),
            reason,
        )
        .await?;
    }

    Ok(HttpResponse::Ok().json(new_resource))
}

#[actix_web::delete("/{id}")]
async fn delete(_cur_user: CurUser, id: web::Path<Uuid>) -> ApiResult {
    let resource = DeploymentResource::find(id.into_inner()).await?;
    let resource_type = DeploymentResourceType::find(resource.type_id).await?;
    let resource_spec = resource_type.spec()?;
    // TODO: Check allowed_role
    if let Some(target) = resource_spec.lifecycle.delete.target.as_ref() {
        resource.sync_to(target).await?;
    }
    resource.delete().await?;
    Ok(HttpResponse::NoContent().finish())
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/deployment-resources")
            .service(get_all)
            .service(get)
            .service(create)
            .service(update)
            .service(delete),
    );
}