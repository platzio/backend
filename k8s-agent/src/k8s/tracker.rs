use super::annotations::{
    find_deployment_from_namespace, DEPLOYMENT_NAMESPACE_LABELS_SELECTOR,
    NAMESPACE_ANNOTATION_DEPLOYMENT_ID,
};
use super::cluster_type::K8s;
use anyhow::{anyhow, Result};
use chrono::prelude::*;
use futures::{FutureExt, StreamExt, TryStreamExt};
use kube::api::{Api, ListParams, WatchEvent};
use kube::ResourceExt;
use lazy_static::lazy_static;
use log::*;
use platz_db::{DeploymentStatus, K8sResource, NewK8sCluster, UpdateK8sClusterStatus};
use platz_sdk::deployment_status::StatusColor;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{broadcast, watch, RwLock};
use tokio::{select, task, time};
use uuid::Uuid;

#[derive(Clone)]
pub struct K8sTracker {
    inner: Arc<RwLock<Inner>>,
}

struct Inner {
    inbound_requests_tx: broadcast::Sender<Arc<K8s>>,
    outbound_notifications_tx: watch::Sender<()>,
    outbound_notifications_rx: watch::Receiver<()>,
    clusters: HashMap<Uuid, Arc<K8s>>,
    tasks: HashMap<Uuid, task::JoinHandle<()>>,
}

lazy_static! {
    pub static ref K8S_TRACKER: K8sTracker = K8sTracker::new();
}

impl K8sTracker {
    pub fn new() -> Self {
        let (inbound_requests_tx, _) = broadcast::channel(64);
        let (outbound_notifications_tx, outbound_notifications_rx) = watch::channel(());
        let this = Self {
            inner: Arc::new(RwLock::new(Inner {
                inbound_requests_tx,
                outbound_notifications_tx,
                outbound_notifications_rx,
                clusters: Default::default(),
                tasks: Default::default(),
            })),
        };
        let this2 = this.clone();
        task::spawn(async move { this2.run().await });
        this
    }

    pub async fn inbound_requests_tx(&self) -> broadcast::Sender<Arc<K8s>> {
        self.inner.read().await.inbound_requests_tx.clone()
    }

    pub async fn outbound_notifications_rx(&self) -> watch::Receiver<()> {
        self.inner.read().await.outbound_notifications_rx.clone()
    }

    pub async fn get_ids(&self) -> Vec<Uuid> {
        self.inner.read().await.clusters.keys().copied().collect()
    }

    pub async fn get_cluster(&self, id: Uuid) -> Result<Arc<K8s>> {
        let reader = self.inner.read().await;
        match reader.clusters.get(&id) {
            None => Err(anyhow!("Could not find cluster {}", id)),
            Some(cluster) => Ok(cluster.clone()),
        }
    }

    async fn run(&self) {
        let mut rx = self.inner.read().await.inbound_requests_tx.subscribe();
        while let Ok(cluster) = rx.recv().await {
            let db_cluster = match NewK8sCluster::from(cluster.as_ref()).insert().await {
                Ok(db_cluster) => {
                    debug!("  Updated in database: {:?}", db_cluster);
                    db_cluster
                }
                Err(err) => {
                    error!("  Failed updating cluster in database: {:?}", err);
                    continue;
                }
            };
            if db_cluster.ignore {
                self.stop_watching_cluster(db_cluster.id).await
            } else {
                self.start_watching_cluster(db_cluster.id, cluster).await
            }
            self.inner
                .read()
                .await
                .outbound_notifications_tx
                .send(())
                .unwrap();
        }
    }

    async fn start_watching_cluster(&self, id: Uuid, cluster: Arc<K8s>) {
        let mut inner = self.inner.write().await;
        inner.clusters.insert(id, cluster);

        inner.tasks.entry(id).or_insert_with(|| {
            let self_ = self.clone();
            task::spawn(async move {
                loop {
                    debug!("Starting cluster watch task for {}", id);
                    match self_.watch_cluster(id).await {
                        Ok(_) => (),
                        Err(err) => {
                            error!("Cluster watch task {} finished with error: {:?}", id, err);
                        }
                    }
                    time::sleep(time::Duration::from_secs(5)).await;
                }
            })
        });
    }

    async fn stop_watching_cluster(&self, id: Uuid) {
        let mut inner = self.inner.write().await;
        inner.clusters.remove(&id);
        match inner.tasks.remove(&id) {
            None => warn!(
                "Requested to stop watching cluster {}, but it wasn't being watched",
                id
            ),
            Some(handle) => {
                info!("Stopping cluster {} watch", id);
                handle.abort();
            }
        }
    }

    async fn watch_cluster(&self, cluster_id: Uuid) -> Result<()> {
        debug!("Starting to watch cluster {}", cluster_id);

        let client = self
            .inner
            .read()
            .await
            .clusters
            .get(&cluster_id)
            .unwrap()
            .kube_client()
            .await?;

        handle_already_cleared_namespaces(cluster_id, client.clone()).await?;

        set_cluster_status(cluster_id, true, None).await?;
        match watch_for_cluster_changes(cluster_id, client).await {
            Ok(_) => Ok(()),
            Err(err) => {
                set_cluster_status(cluster_id, false, Some(err.to_string())).await?;
                Err(err)
            }
        }
    }
}

async fn watch_for_cluster_changes(cluster_id: Uuid, client: kube::Client) -> Result<()> {
    let start_time = Utc::now();
    let ns_api = Api::<k8s_openapi::api::core::v1::Namespace>::all(client.clone());
    let mut namespaces = Api::<k8s_openapi::api::core::v1::Namespace>::all(client.clone())
        .watch(
            &ListParams::default().labels(&DEPLOYMENT_NAMESPACE_LABELS_SELECTOR),
            "0",
        )
        .await?
        .boxed();
    let mut deployments = Api::<k8s_openapi::api::apps::v1::Deployment>::all(client.clone())
        .watch(&ListParams::default(), "0")
        .await?
        .boxed();
    let mut statefulsets = Api::<k8s_openapi::api::apps::v1::StatefulSet>::all(client.clone())
        .watch(&ListParams::default(), "0")
        .await?
        .boxed();
    let mut jobs = Api::<k8s_openapi::api::batch::v1::Job>::all(client)
        .watch(&ListParams::default(), "0")
        .await?
        .boxed();

    // Delete unfamiliar resources after 1 minute of successfully getting updates from k8s
    let mut delete_resources_timeout = time::sleep(time::Duration::from_secs(60)).fuse().boxed();

    loop {
        select! {
            result = namespaces.try_next() => {
                match result? {
                    Some(event) => {
                        info!("Namespace event in cluster {}: {:?}", cluster_id, event);
                        handle_namespace_event(event, ).await?;
                    }
                    None => break,
                }
            }
            result = deployments.try_next() => {
                match result? {
                    Some(event) => {
                        info!("Deployment event in cluster {}: {:?}", cluster_id, event);
                        handle_resource_event(cluster_id, event, &ns_api, k8s_deployment_status).await?;
                    }
                    None => break,
                }
            }
            result = statefulsets.try_next() => {
                match result? {
                    Some(event) => {
                        info!("Statefulset event in cluster {}: {:?}", cluster_id, event);
                        handle_resource_event(cluster_id, event, &ns_api, k8s_statefulset_status).await?;
                    }
                    None => break,
                }
            }
            result = jobs.try_next() => {
                match result? {
                    Some(event) => {
                        info!("Job event in cluster {}: {:?}", cluster_id, event);
                        handle_resource_event(cluster_id, event, &ns_api, k8s_job_status).await?;
                    }
                    None => break,
                }
            }
            _ = delete_resources_timeout.as_mut() => {
                info!("Deleting old K8sResources");
                for resource in K8sResource::find_older_than(cluster_id, start_time).await? {
                    debug!("Deleting {:?}", resource);
                    resource.delete().await?;
                }
            }
        }
    }

    Ok(())
}

pub async fn handle_already_cleared_namespaces(
    cluster_id: Uuid,
    client: kube::Client,
) -> Result<()> {
    let managed_namespaces = Api::<k8s_openapi::api::core::v1::Namespace>::all(client.clone())
        .list(&ListParams::default().labels(&DEPLOYMENT_NAMESPACE_LABELS_SELECTOR))
        .await?;

    let existing_deployment_ids = HashSet::<String>::from_iter(
        managed_namespaces
            .iter()
            .filter_map(|x| x.annotations().get(NAMESPACE_ANNOTATION_DEPLOYMENT_ID))
            .cloned(),
    );

    let deployments =
        platz_db::Deployment::all_with_ongoing_clearing_status_in_cluster(cluster_id).await?;

    for deployment in deployments {
        debug!(
            "handle_already_cleared_namespaces is examining {} ({})",
            deployment.namespace_name(),
            deployment.id
        );

        if !existing_deployment_ids.contains(&deployment.id.to_string()) {
            warn!(
                "Found deployment {} with status {} with no living namespace, considering its clearance as completed",                
                deployment.namespace_name(),
                deployment.status
            );
            deployment_removal_completed(deployment).await?;
        }
    }

    Ok(())
}

async fn handle_namespace_event(
    event: WatchEvent<k8s_openapi::api::core::v1::Namespace>,
) -> Result<()> {
    match event {
        WatchEvent::Added(ns) | WatchEvent::Modified(ns) => {
            match find_deployment_from_namespace(&ns).await? {
                Some(_deployment) => {}
                None => {
                    // TODO: Alert
                    error!(
                        "Found an annotated namespace but not its deployment: {:?}",
                        ns
                    );
                }
            };
            Ok(())
        }
        WatchEvent::Deleted(ns) => {
            match find_deployment_from_namespace(&ns).await? {
                Some(deployment) => {
                    deployment_removal_completed(deployment).await?;
                }
                None => {
                    // TODO: Alert
                    error!(
                        "Found an annotated namespace but not its deployment: {:?}",
                        ns
                    );
                }
            };
            Ok(())
        }
        WatchEvent::Bookmark(_) => Ok(()),
        WatchEvent::Error(err) => Err(err.into()),
    }
}

async fn deployment_removal_completed(deployment: platz_db::Deployment) -> Result<()> {
    // Only delete the deployment if the status is Deleting, otherwise
    // this namespace deletion is part of a rename.
    match deployment.status {
        DeploymentStatus::Uninstalling => {
            deployment
                .set_status(DeploymentStatus::Uninstalled, None)
                .await?;
        }
        DeploymentStatus::Deleting => deployment.delete().await?,
        _ => (),
    };

    Ok(())
}

async fn handle_resource_event<T, G>(
    cluster_id: Uuid,
    event: WatchEvent<T>,
    ns_api: &Api<k8s_openapi::api::core::v1::Namespace>,
    get_status_color: G,
) -> Result<()>
where
    T: k8s_openapi::Resource
        + k8s_openapi::Metadata<Ty = k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta>
        + std::fmt::Debug,
    G: Fn(&T) -> Vec<StatusColor>,
{
    let (resource, is_create) = match event {
        WatchEvent::Added(resource) | WatchEvent::Modified(resource) => (resource, true),
        WatchEvent::Deleted(resource) => (resource, false),
        WatchEvent::Bookmark(_) => return Ok(()),
        WatchEvent::Error(err) => return Err(err.into()),
    };

    let metadata = resource.metadata();

    let namespace = match metadata.namespace.as_ref() {
        Some(ns) => ns_api.get(ns).await?,
        None => {
            warn!(
                "[cluster {}] Resource has no namespace: {:?}",
                cluster_id, resource
            );
            return Ok(());
        }
    };

    let deployment = match find_deployment_from_namespace(&namespace).await? {
        None => {
            warn!(
                "[cluster {}] Could not find deployment for namespace {:?}",
                cluster_id, namespace
            );
            return Ok(());
        }
        Some(deployment) => deployment,
    };

    let id = Uuid::parse_str(
        metadata
            .uid
            .as_ref()
            .ok_or_else(|| anyhow!("Resource has no uid"))?,
    )?;

    if is_create {
        K8sResource {
            id,
            cluster_id,
            deployment_id: deployment.id,
            kind: k8s_openapi::kind(&resource).to_owned(),
            api_version: k8s_openapi::api_version(&resource).to_owned(),
            name: metadata
                .name
                .as_ref()
                .ok_or_else(|| {
                    anyhow!(
                        "[cluster {}] Resource has no name: {:?}",
                        cluster_id,
                        resource
                    )
                })?
                .to_owned(),
            status_color: get_status_color(&resource)
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            metadata: serde_json::to_value(metadata)?,
            last_updated_at: Utc::now(),
        }
        .save()
        .await?;
    } else {
        match K8sResource::delete_by_id(id).await {
            Ok(_) => debug!("[cluster {}] Deleted K8sResource {}", cluster_id, id),
            Err(err) => error!(
                "[cluster {}] Failed deleting K8sResource {}: {:?}",
                cluster_id, id, err,
            ),
        }
    }

    Ok(())
}

async fn set_cluster_status(id: Uuid, is_ok: bool, reason: Option<String>) -> Result<()> {
    UpdateK8sClusterStatus {
        is_ok: Some(is_ok),
        not_ok_reason: Some(reason),
    }
    .save(id)
    .await?;
    Ok(())
}

fn k8s_deployment_status(deployment: &k8s_openapi::api::apps::v1::Deployment) -> Vec<StatusColor> {
    let status = match &deployment.status {
        Some(status) => status,
        None => return Vec::new(),
    };

    let available = status.available_replicas.unwrap_or_default() as usize;
    let unavailable = status.unavailable_replicas.unwrap_or_default() as usize;

    std::iter::repeat(StatusColor::Success)
        .take(available)
        .chain(std::iter::repeat(StatusColor::Danger).take(unavailable))
        .collect()
}

fn k8s_statefulset_status(
    statefulset: &k8s_openapi::api::apps::v1::StatefulSet,
) -> Vec<StatusColor> {
    let status = match &statefulset.status {
        Some(status) => status,
        None => return Vec::new(),
    };

    let replicas = status.replicas as usize;
    let ready = status.ready_replicas.unwrap_or_default() as usize;

    std::iter::repeat(StatusColor::Success)
        .take(ready)
        .chain(std::iter::repeat(StatusColor::Danger).take(replicas - ready))
        .collect()
}

fn k8s_job_status(job: &k8s_openapi::api::batch::v1::Job) -> Vec<StatusColor> {
    let status = match &job.status {
        Some(status) => status,
        None => return Vec::new(),
    };

    std::iter::repeat(StatusColor::Primary)
        .take(status.active.unwrap_or_default() as usize)
        .chain(
            std::iter::repeat(StatusColor::Success)
                .take(status.succeeded.unwrap_or_default() as usize),
        )
        .chain(
            std::iter::repeat(StatusColor::Danger).take(status.failed.unwrap_or_default() as usize),
        )
        .collect()
}
