use anyhow::{anyhow, Result};
use log::*;
use platz_chart_ext::resource_types::v1::{ChartExtResourceLifecycle, ResourceLifecycle};
use platz_db::{
    db_events, DbEventOperation, DbTable, DeploymentResource, DeploymentResourceType, SyncStatus,
    UpdateDeploymentResourceSyncStatus,
};

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
        SyncStatus::Creating => {
            info!("Creating {} ({})", resource.id, resource.name);
            call_lifecycle_target(&resource, |lifecycle| &lifecycle.create).await?;
        }
        SyncStatus::Updating => {
            info!("Updating {} ({})", resource.id, resource.name);
            call_lifecycle_target(&resource, |lifecycle| &lifecycle.update).await?;
        }
        SyncStatus::Deleting => {
            info!("Deleting {} ({})", resource.id, resource.name);
            if call_lifecycle_target(&resource, |lifecycle| &lifecycle.delete).await? {
                resource.delete().await?;
            }
        }
        SyncStatus::Ready | SyncStatus::Error => {
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
    F: FnOnce(&ChartExtResourceLifecycle) -> &ResourceLifecycle,
{
    let resource_type = DeploymentResourceType::find(resource.type_id).await?;
    let resource_spec = resource_type.spec()?;
    let lifecycle = get_lifecycle_action(&resource_spec.lifecycle);
    match lifecycle.target.as_ref() {
        None => Ok(true),
        Some(target) => match resource.sync_to(target).await {
            Ok(result) => {
                debug!(
                    "Sync of {} ({}) completed: {}",
                    resource.id, resource.name, result
                );
                UpdateDeploymentResourceSyncStatus {
                    sync_status: Some(SyncStatus::Ready),
                    sync_reason: Some(None),
                }
                .save(resource.id)
                .await?;
                Ok(true)
            }
            Err(err) => {
                error!("Error syncing {} ({}): {}", resource.id, resource.name, err);
                UpdateDeploymentResourceSyncStatus {
                    sync_status: Some(SyncStatus::Error),
                    sync_reason: Some(Some(err.to_string())),
                }
                .save(resource.id)
                .await?;
                Ok(false)
            }
        },
    }
}
