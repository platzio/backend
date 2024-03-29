use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use platz_auth::ApiIdentity;
use platz_db::{
    DbError, DbTableOrDeploymentResource, Deployment, DeploymentTask, DeploymentTaskOperation,
    HelmChart, Json, K8sCluster, K8sResource, NewDeploymentTask,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct Query {
    deployment_id: Option<Uuid>,
}

async fn get_all(query: web::Query<Query>) -> ApiResult {
    Ok(HttpResponse::Ok().json(match query.deployment_id {
        Some(deployment_id) => DeploymentTask::find_by_deployment_id(deployment_id).await?,
        None => DeploymentTask::all().await?,
    }))
}

async fn get(id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(DeploymentTask::find(id.into_inner()).await?))
}

#[derive(Debug, Deserialize)]
pub struct ApiNewDeploymentTask {
    pub deployment_id: Uuid,
    pub operation: DeploymentTaskOperation,
}

async fn create(identity: ApiIdentity, task: web::Json<ApiNewDeploymentTask>) -> ApiResult {
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
        execute_at: None,
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

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(get_all));
    cfg.route("/{id}", web::get().to(get));
    cfg.route("", web::post().to(create));
}
