use crate::permissions::verify_deployment_maintainer;
use crate::permissions::verify_deployment_owner;
use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use platz_auth::ApiIdentity;
use platz_chart_ext::ChartExtCardinality;
use platz_db::{
    DbTable, DbTableOrDeploymentResource, Deployment, DeploymentKind, DeploymentStatus,
    DeploymentTask, HelmChart, NewDeployment, UpdateDeployment,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct Query {
    #[serde(default, deserialize_with = "crate::serde_utils::bool_from_str")]
    all: bool,
    kind: Option<DeploymentKind>,
    cluster_id: Option<Uuid>,
}

async fn get_all(query: web::Query<Query>) -> ApiResult {
    let deployments = match query.kind.as_ref() {
        None => Deployment::all().await?,
        Some(kind) => Deployment::find_by_kind(kind.to_owned()).await?,
    };
    let deployments = match query.all {
        false => deployments
            .into_iter()
            .filter(|deployment| deployment.enabled)
            .collect(),
        true => deployments,
    };
    let deployments = match query.cluster_id {
        None => deployments,
        Some(cluster_id) => deployments
            .into_iter()
            .filter(|deployment| deployment.cluster_id == cluster_id)
            .collect(),
    };
    Ok(HttpResponse::Ok().json(deployments))
}

async fn get(id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(Deployment::find(id.into_inner()).await?))
}

async fn create(identity: ApiIdentity, new_deployment: web::Json<NewDeployment>) -> ApiResult {
    let new_deployment = new_deployment.into_inner();
    verify_deployment_owner(new_deployment.cluster_id, &new_deployment.kind, &identity).await?;

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

fn using_error(prefix: &str, deployments: Vec<Deployment>) -> String {
    format!(
        "{}: {}",
        prefix,
        deployments
            .into_iter()
            .map(|d| format!("{} ({})", d.name, d.kind))
            .collect::<Vec<String>>()
            .join(", ")
    )
}

async fn update(
    identity: ApiIdentity,
    id: web::Path<Uuid>,
    data: web::Json<UpdateDeployment>,
) -> ApiResult {
    let updates = data.into_inner();
    let old_deployment = Deployment::find(id.into_inner()).await?;

    verify_deployment_maintainer(old_deployment.cluster_id, &old_deployment.kind, &identity)
        .await?;

    if let Some(new_cluster_id) = updates.cluster_id {
        if new_cluster_id != old_deployment.cluster_id {
            verify_deployment_maintainer(new_cluster_id, &old_deployment.kind, &identity).await?;
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

async fn delete(identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    let deployment = Deployment::find(id.into_inner()).await?;
    verify_deployment_owner(deployment.cluster_id, &deployment.kind, &identity).await?;

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

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(get_all));
    cfg.route("/{id}", web::get().to(get));
    cfg.route("", web::post().to(create));
    cfg.route("/{id}", web::put().to(update));
    cfg.route("/{id}", web::delete().to(delete));
}
