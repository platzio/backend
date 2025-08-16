use super::utils::ensure_user;
use crate::permissions::verify_deployment_maintainer;
use crate::result::ApiResult;
use actix_web::{HttpResponse, delete, get, post, web};
use chrono::prelude::*;
use platz_auth::ApiIdentity;
use platz_db::{
    DbError, DbTableOrDeploymentResource, Json,
    diesel_pagination::{Paginated, PaginationParams},
    schema::{
        deployment::Deployment,
        deployment_task::{
            DeploymentTask, DeploymentTaskExtraFilters, DeploymentTaskFilters,
            DeploymentTaskOperation, NewDeploymentTask,
        },
        helm_chart::HelmChart,
        k8s_cluster::K8sCluster,
        k8s_resource::K8sResource,
    },
};
use serde::Deserialize;
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Deployment Tasks",
    operation_id = "allDeploymentTasks",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    params(DeploymentTaskFilters),
    responses(
        (
            status = OK,
            body = Paginated<DeploymentTask>,
        ),
    ),
)]
#[get("/deployment-tasks")]
async fn get_all(
    _identity: ApiIdentity,
    filters: web::Query<DeploymentTaskFilters>,
    extra_filters: web::Query<DeploymentTaskExtraFilters>,
    pagination: web::Query<PaginationParams>,
) -> ApiResult {
    Ok(HttpResponse::Ok().json(
        DeploymentTask::all_filtered(
            filters.into_inner(),
            extra_filters.into_inner(),
            pagination.into_inner(),
        )
        .await?,
    ))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Deployment Tasks",
    operation_id = "getDeploymentTask",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    responses(
        (
            status = OK,
            body = DeploymentTask,
        ),
    ),
)]
#[get("/deployment-tasks/{id}")]
async fn get_one(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(DeploymentTask::find(id.into_inner()).await?))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CancelDeploymentTask {
    #[schema(required)]
    pub reason: Option<String>,
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Deployment Tasks",
    operation_id = "cancelDeploymentTask",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    request_body = CancelDeploymentTask,
    responses(
        (
            status = OK,
            body = DeploymentTask,
        ),
    ),
)]
#[delete("/deployment-tasks/{id}")]
async fn cancel_one(
    identity: ApiIdentity,
    id: web::Path<Uuid>,
    body: web::Json<CancelDeploymentTask>,
) -> ApiResult {
    let body = body.into_inner();
    let task = DeploymentTask::find(id.into_inner()).await?;
    let canceled_by_user_id = identity.inner().user_id();
    let canceled_by_deployment_id = identity.inner().deployment_id();

    if task.acting_user_id != canceled_by_user_id
        && task.acting_deployment_id != canceled_by_deployment_id
        && !ensure_user(&identity).await?.is_admin
    {
        return Ok(HttpResponse::Forbidden().json(json!({
            "message": "Only admin users can cancel tasks of other users",
        })));
    }

    if task.execute_at < Utc::now() + chrono::Duration::minutes(5) {
        return Ok(HttpResponse::Forbidden().json(json!({
            "message": "Cannot cancel task which is closed to being executed",
        })));
    }

    let task = platz_db::schema::deployment_task::CancelDeploymentTask {
        canceled_by_user_id,
        canceled_by_deployment_id,
        reason: body.reason,
    }
    .save(task.id)
    .await?;

    Ok(HttpResponse::Ok().json(task))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateDeploymentTask {
    pub deployment_id: Uuid,
    pub operation: DeploymentTaskOperation,
    pub execute_at: Option<DateTime<Utc>>,
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Deployment Tasks",
    operation_id = "createDeploymentTask",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    request_body = CreateDeploymentTask,
    responses(
        (
            status = CREATED,
            body = DeploymentTask,
        ),
    ),
)]
#[post("/deployment-tasks")]
async fn create(identity: ApiIdentity, task: web::Json<CreateDeploymentTask>) -> ApiResult {
    let task = task.into_inner();

    let deployment = Deployment::find(task.deployment_id).await?;

    let cluster = K8sCluster::find(deployment.cluster_id).await?;
    let env_id = match cluster.env_id {
        Some(env_id) => env_id,
        None => return Ok(HttpResponse::InternalServerError().json(json!({
            "error": "Can't create a deployment task for a deployment in a cluster with no env_id",
        }))),
    };

    let task = NewDeploymentTask {
        cluster_id: cluster.id,
        deployment_id: task.deployment_id,
        acting_user_id: identity.inner().user_id(),
        acting_deployment_id: identity.inner().deployment_id(),
        operation: Json(task.operation),
        status: Default::default(),
        execute_at: task.execute_at,
    };

    Ok(match &task.operation {
        Json(DeploymentTaskOperation::InvokeAction(params)) => {
            if params.helm_chart_id == deployment.helm_chart_id {
                let chart = HelmChart::find(params.helm_chart_id).await?;
                let actions_schema = chart.actions_schema()?;
                let action_schema = actions_schema
                    .find(&params.action_id)
                    .ok_or_else(|| DbError::HelmChartNoSuchAction(params.action_id.to_owned()))?;
                match action_schema
                    .generate_body::<DbTableOrDeploymentResource>(env_id, params.body.clone())
                    .await
                {
                    Ok(_) => HttpResponse::Created().json(task.insert().await?),
                    Err(err) => HttpResponse::BadRequest().json(json!({
                        "message": err.to_string()
                    })),
                }
            } else {
                HttpResponse::Conflict().json(json!({
                   "message": "helm_chart_id of action invocation must match the current helm_chart_id of the deployment"
               }))
            }
        }
        Json(DeploymentTaskOperation::RestartK8sResource(params)) => {
            verify_deployment_maintainer(deployment.cluster_id, deployment.kind_id, &identity)
                .await?;
            match K8sResource::find(params.resource_id).await? {
                None => HttpResponse::NotFound().json(json!({
                    "message": format!("Unknown resource with id={}", params.resource_id)
                })),
                Some(resource) => {
                    if params.resource_name != resource.name {
                        HttpResponse::BadRequest().json(json!({
                            "message": format!("Resource name to restart doesn't match: Provided \"{}\", actual \"{}\" (id={})",
                                params.resource_name, resource.name, resource.id)
                        }))
                    } else {
                        HttpResponse::Created().json(task.insert().await?)
                    }
                }
            }
        }
        _ => HttpResponse::Forbidden().json(json!({
            "message":
                format!(
                    "{:?} tasks can't be created through the API",
                    task.operation
                )
        })),
    })
}

#[derive(utoipa::OpenApi)]
#[openapi(
    tags((
        name = "Deployment Tasks",
        description = "\
Deployment tasks are all operations performed on each deployment, along with
their status.
        ",
    )),
    paths(get_all, get_one, create),
)]
pub(super) struct OpenApi;
