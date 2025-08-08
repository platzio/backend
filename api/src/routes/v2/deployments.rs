use crate::{
    permissions::{verify_deployment_maintainer, verify_deployment_owner},
    result::ApiResult,
};
use actix_web::{delete, get, post, put, web, HttpResponse};
use platz_auth::ApiIdentity;
use platz_chart_ext::ChartExtCardinality;
use platz_db::{
    diesel_pagination::{Paginated, PaginationParams},
    schema::{
        deployment::{
            Deployment, DeploymentExtraFilters, DeploymentFilters, DeploymentStatus, NewDeployment,
            UpdateDeployment,
        },
        deployment_task::DeploymentTask,
        helm_chart::HelmChart,
    },
    DbTable, DbTableOrDeploymentResource,
};
use serde_json::json;
use uuid::Uuid;

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Deployments",
    operation_id = "allDeployments",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    params(DeploymentFilters),
    responses(
        (
            status = OK,
            body = Paginated<Deployment>,
        ),
    ),
)]
#[get("/deployments")]
async fn get_all(
    _identity: ApiIdentity,
    filters: web::Query<DeploymentFilters>,
    extra_filters: web::Query<DeploymentExtraFilters>,
    pagination: web::Query<PaginationParams>,
) -> ApiResult {
    Ok(HttpResponse::Ok().json(
        Deployment::all_filtered(
            filters.into_inner(),
            extra_filters.into_inner(),
            pagination.into_inner(),
        )
        .await?,
    ))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Deployments",
    operation_id = "getDeployment",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    responses(
        (
            status = OK,
            body = Deployment,
        ),
    ),
)]
#[get("/deployments/{id}")]
async fn get_one(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(Deployment::find(id.into_inner()).await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Deployments",
    operation_id = "createDeployment",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    request_body = NewDeployment,
    responses(
        (
            status = CREATED,
            body = Deployment,
        ),
    ),
)]
#[post("/deployments")]
async fn create(identity: ApiIdentity, new_deployment: web::Json<NewDeployment>) -> ApiResult {
    let new_deployment = new_deployment.into_inner();
    verify_deployment_owner(new_deployment.cluster_id, new_deployment.kind_id, &identity).await?;

    let chart = HelmChart::find(new_deployment.helm_chart_id).await?;
    match chart.features()?.cardinality() {
        ChartExtCardinality::Many => {
            if new_deployment.name.is_empty() {
                return Ok(HttpResponse::BadRequest().json(json!({
                    "error": "Missing name field",
                })));
            }
        }
        ChartExtCardinality::OnePerCluster => {
            if !new_deployment.name.is_empty() {
                return Ok(HttpResponse::Conflict().json(json!({
                    "error": "This is a one per cluster deployment, therefore is cannot be assigned a name",
                })));
            }
        }
    }

    let deployment = new_deployment.insert().await?;
    DeploymentTask::create_install_task(&deployment, &identity).await?;
    Ok(HttpResponse::Created().json(deployment))
}

pub fn using_error(prefix: &str, deployments: Vec<Deployment>) -> String {
    format!(
        "{}: {}",
        prefix,
        deployments
            .into_iter()
            .map(|d| format!("{} (Kind ID: {})", d.name, d.kind_id))
            .collect::<Vec<String>>()
            .join(", ")
    )
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Deployments",
    operation_id = "updateDeployment",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    request_body = UpdateDeployment,
    responses(
        (
            status = OK,
            body = Deployment,
        ),
    ),
)]
#[put("/deployments/{id}")]
async fn update(
    identity: ApiIdentity,
    id: web::Path<Uuid>,
    data: web::Json<UpdateDeployment>,
) -> ApiResult {
    let updates = data.into_inner();
    let old_deployment = Deployment::find(id.into_inner()).await?;

    verify_deployment_maintainer(old_deployment.cluster_id, old_deployment.kind_id, &identity)
        .await?;

    if let Some(new_cluster_id) = updates.cluster_id {
        if new_cluster_id != old_deployment.cluster_id {
            verify_deployment_maintainer(new_cluster_id, old_deployment.kind_id, &identity).await?;
        }
    }

    if old_deployment.enabled && updates.enabled == Some(false) {
        let dependents: Vec<_> = Deployment::find_using(
            &DbTableOrDeploymentResource::DbTable(DbTable::Deployments),
            old_deployment.id,
        )
        .await?
        .into_iter()
        .filter(|d| d.enabled)
        .collect();
        if !dependents.is_empty() {
            return Ok(HttpResponse::Conflict().json(json!({
                        "message": using_error("This deployment can't be disabled because other deployments depend on it", dependents),
                    })));
        }
    }

    let new_deployment = updates.save(old_deployment.id).await?;
    let chart = HelmChart::find(new_deployment.helm_chart_id).await?;
    let features = chart.features()?;

    if new_deployment.enabled {
        let mut reinstall_dependencies = false;

        if !old_deployment.enabled {
            DeploymentTask::create_install_task(&new_deployment, &identity).await?;
            reinstall_dependencies = true;
        } else if (old_deployment.cluster_id != new_deployment.cluster_id)
            || (old_deployment.name != new_deployment.name)
        {
            DeploymentTask::create_recreate_task(&old_deployment, &new_deployment, &identity)
                .await?;
            DeploymentTask::create_upgrade_task(&old_deployment, &new_deployment, &identity)
                .await?;
            reinstall_dependencies = true;
        } else if (old_deployment.config != new_deployment.config)
            || (old_deployment.helm_chart_id != new_deployment.helm_chart_id)
            || (old_deployment.values_override != new_deployment.values_override)
        {
            DeploymentTask::create_upgrade_task(&old_deployment, &new_deployment, &identity)
                .await?;
            reinstall_dependencies = features.reinstall_dependencies();
        }

        if reinstall_dependencies {
            Deployment::reinstall_all_using(
                &DbTableOrDeploymentResource::DbTable(DbTable::Deployments),
                new_deployment.id,
                &identity,
                format!("The {} deployment has been updated", old_deployment.name),
            )
            .await?;
        }
    } else if old_deployment.enabled {
        DeploymentTask::create_uninstall_task(&old_deployment, &identity).await?;
    }

    Ok(HttpResponse::Ok().json(new_deployment))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Deployments",
    operation_id = "deleteDeployment",
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
#[delete("/deployments/{id}")]
async fn delete(identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    let deployment = Deployment::find(id.into_inner()).await?;
    verify_deployment_owner(deployment.cluster_id, deployment.kind_id, &identity).await?;

    let dependents = Deployment::find_using(
        &DbTableOrDeploymentResource::DbTable(DbTable::Deployments),
        deployment.id,
    )
    .await?;
    if !dependents.is_empty() {
        return Ok(HttpResponse::Conflict().json(json!({
            "message": using_error("This deployment can't be deleted because other deployments depend on it", dependents),
        })));
    }

    deployment
        .set_status(DeploymentStatus::Deleting, None)
        .await?;

    DeploymentTask::create_uninstall_task(&deployment, &identity).await?;

    Ok(HttpResponse::NoContent().finish())
}

#[derive(utoipa::OpenApi)]
#[openapi(
    tags((
        name = "Deployments",
        description = "\
This collection contains deployments of Helm chart into envs.
",
    )),
    paths(get_all, get_one, create, update, delete),
)]
pub(super) struct OpenApi;
