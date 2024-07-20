use anyhow::{anyhow, Result};
use platz_chart_ext::resource_types::{
    ChartExtResourceLifecycleActionV1Beta1, ChartExtResourceLifecycleV1Beta1,
};
use platz_db::{
    db_events, DbEventOperation, DbTable, DeploymentResource, DeploymentResourceSyncStatus,
    DeploymentResourceType, UpdateDeploymentResourceSyncStatus,
};
use tracing::{debug, error, info};

pub async fn monitor_deployment_resource_changes() -> Result<()> {
    let mut db_rx = db_events();
    while let Ok(event) = db_rx.recv().await {
        debug!("Got {:?}", event);
        if event.table == DbTable::DeploymentResources {
            match event.operation {
                DbEventOperation::Delete => (),
                _ => {
                    let resource = DeploymentResource::find(event.data.id).await?;
                    sync_resource(resource).await?;
                }
            }
        }
    }

    Err(anyhow!("monitor_account_permission_changes returned"))
}

pub async fn scrub_deployment_resources() -> Result<()> {
    for resource in DeploymentResource::all().await? {
        sync_resource(resource).await?;
    }
    Ok(())
}

async fn sync_resource(resource: DeploymentResource) -> Result<()> {
    info!(
        "Checking sync for resource {} ({})",
        resource.id, resource.name
    );

    match resource.sync_status {
        DeploymentResourceSyncStatus::Creating => {
            info!("Creating {} ({})", resource.id, resource.name);
            call_lifecycle_target(&resource, |lifecycle| lifecycle.create.as_ref()).await?;
        }
        DeploymentResourceSyncStatus::Updating => {
            info!("Updating {} ({})", resource.id, resource.name);
            call_lifecycle_target(&resource, |lifecycle| lifecycle.update.as_ref()).await?;
        }
        DeploymentResourceSyncStatus::Deleting => {
            info!("Deleting {} ({})", resource.id, resource.name);
            if call_lifecycle_target(&resource, |lifecycle| lifecycle.delete.as_ref()).await? {
                resource.delete().await?;
            }
        }
        DeploymentResourceSyncStatus::Ready | DeploymentResourceSyncStatus::Error => {
            debug!("Nothing to do for {} ({})", resource.id, resource.name);
        }
    }
    Ok(())
}

async fn call_lifecycle_target<F>(
    resource: &DeploymentResource,
    get_lifecycle_action: F,
) -> Result<bool>
where
    F: FnOnce(&ChartExtResourceLifecycleV1Beta1) -> Option<&ChartExtResourceLifecycleActionV1Beta1>,
{
    let resource_type = DeploymentResourceType::find(resource.type_id).await?;
    let resource_spec = resource_type.spec()?;
    let lifecycle = get_lifecycle_action(&resource_spec.lifecycle);
    match lifecycle
        .and_then(|lifecycle| lifecycle.target.as_ref())
        .as_ref()
    {
        None => {
            debug!(
                "Resource type for {} ({}) has no lifecycle hook, setting as ready",
                resource.id, resource.name,
            );
            UpdateDeploymentResourceSyncStatus {
                sync_status: Some(DeploymentResourceSyncStatus::Ready),
                sync_reason: Some(None),
            }
            .save(resource.id)
            .await?;
            Ok(true)
        }
        Some(target) => match resource.sync_to(target).await {
            Ok(result) => {
                debug!(
                    "Sync of {} ({}) completed: {}",
                    resource.id, resource.name, result
                );
                UpdateDeploymentResourceSyncStatus {
                    sync_status: Some(DeploymentResourceSyncStatus::Ready),
                    sync_reason: Some(None),
                }
                .save(resource.id)
                .await?;
                Ok(true)
            }
            Err(err) => {
                error!("Error syncing {} ({}): {}", resource.id, resource.name, err);
                UpdateDeploymentResourceSyncStatus {
                    sync_status: Some(DeploymentResourceSyncStatus::Error),
                    sync_reason: Some(Some(err.to_string())),
                }
                .save(resource.id)
                .await?;
                Ok(false)
            }
        },
    }
}
