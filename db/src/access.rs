//! Authorization scoping shared by the REST API and the websocket event feed.
//!
//! Both the REST list/detail endpoints and the websocket notification feed must
//! make the *same* decision about which environments an identity is allowed to
//! see. [`AccessScope`] is that single source of truth: it is resolved once per
//! request (or once per websocket connection) and then applied as a SQL filter
//! so the database does the filtering in a single paginated query rather than
//! the application filtering rows one by one.

use crate::{
    DbEvent, DbResult, DbTable, Identity, db_conn,
    schema::{
        deployment::deployments, deployment_resource::deployment_resources,
        deployment_task::deployment_tasks, env_user_permission::env_user_permissions,
        k8s_cluster::k8s_clusters, user::users,
    },
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use std::ops::DerefMut;
use uuid::Uuid;

/// The set of environments an identity is allowed to access.
#[derive(Debug, Clone)]
pub enum AccessScope {
    /// Unrestricted access. Site admins and the service identities (bots and
    /// in-cluster deployments) see everything.
    All,
    /// A regular user, restricted to the environments they have any permission
    /// in. May be empty, in which case the identity sees nothing.
    Envs(Vec<Uuid>),
}

impl AccessScope {
    /// Resolve the scope for an identity with a single small, indexed query.
    pub async fn for_identity(identity: &Identity) -> DbResult<Self> {
        match identity {
            Identity::User(user_id) => {
                let is_admin = users::table
                    .find(user_id)
                    .select(users::is_admin)
                    .get_result::<bool>(db_conn().await?.deref_mut())
                    .await
                    .optional()?
                    .unwrap_or(false);
                if is_admin {
                    return Ok(Self::All);
                }
                let env_ids = env_user_permissions::table
                    .filter(env_user_permissions::user_id.eq(user_id))
                    .select(env_user_permissions::env_id)
                    .get_results::<Uuid>(db_conn().await?.deref_mut())
                    .await?;
                Ok(Self::Envs(env_ids))
            }
            // Service identities are trusted: bots and in-cluster deployments
            // authenticate with their own tokens and operate across envs.
            Identity::Bot(_) | Identity::Deployment(_) => Ok(Self::All),
        }
    }

    /// Whether this scope is unrestricted.
    pub fn is_all(&self) -> bool {
        matches!(self, Self::All)
    }

    /// Whether the identity may access the given (possibly absent) environment.
    /// An object with no environment is never visible to a restricted user.
    pub fn allows_env(&self, env_id: Option<Uuid>) -> bool {
        match self {
            Self::All => true,
            Self::Envs(env_ids) => env_id.is_some_and(|env_id| env_ids.contains(&env_id)),
        }
    }

    /// Decide whether the identity behind this scope is allowed to receive a
    /// websocket [`DbEvent`].
    ///
    /// The environment of the changed row is resolved on demand with a small
    /// primary-key lookup (no cache). Catalog/global tables are visible to any
    /// authenticated identity. For env-scoped rows we fail closed: if the
    /// environment cannot be resolved (for example a `DELETE`, where the row no
    /// longer exists) a restricted user does not receive the event.
    pub async fn can_receive_event(&self, event: &DbEvent) -> DbResult<bool> {
        let env_ids = match self {
            Self::All => return Ok(true),
            Self::Envs(env_ids) => env_ids,
        };

        let row_env_id = match event.table {
            // Global catalog and infrastructure tables: not environment-scoped,
            // so they are visible to every authenticated identity.
            DbTable::HelmTagFormats
            | DbTable::HelmCharts
            | DbTable::HelmRegistries
            | DbTable::DeploymentKinds
            | DbTable::DeploymentResourceTypes
            | DbTable::K8sClusters
            | DbTable::K8sResources
            | DbTable::Users
            | DbTable::Bots
            | DbTable::Settings => return Ok(true),

            DbTable::Envs => Some(event.data.id),
            DbTable::Secrets => secret_env_id(event.data.id).await?,
            DbTable::EnvUserPermissions => env_user_permission_env_id(event.data.id).await?,
            DbTable::DeploymentPermissions => deployment_permission_env_id(event.data.id).await?,
            DbTable::Deployments => deployment_env_id(event.data.id).await?,
            DbTable::DeploymentTasks => deployment_task_env_id(event.data.id).await?,
            DbTable::DeploymentResources => deployment_resource_env_id(event.data.id).await?,
        };

        Ok(row_env_id.is_some_and(|env_id| env_ids.contains(&env_id)))
    }
}

/// Resolve the environment of a k8s cluster (clusters may be detached, hence
/// the nested `Option`).
async fn cluster_env_id(cluster_id: Uuid) -> DbResult<Option<Uuid>> {
    Ok(k8s_clusters::table
        .find(cluster_id)
        .select(k8s_clusters::env_id)
        .get_result::<Option<Uuid>>(db_conn().await?.deref_mut())
        .await
        .optional()?
        .flatten())
}

/// Resolve the environment of a deployment via its cluster.
async fn deployment_env_id(deployment_id: Uuid) -> DbResult<Option<Uuid>> {
    let cluster_id = deployments::table
        .find(deployment_id)
        .select(deployments::cluster_id)
        .get_result::<Uuid>(db_conn().await?.deref_mut())
        .await
        .optional()?;
    match cluster_id {
        Some(cluster_id) => cluster_env_id(cluster_id).await,
        None => Ok(None),
    }
}

/// Resolve the environment of a deployment task via its cluster.
async fn deployment_task_env_id(task_id: Uuid) -> DbResult<Option<Uuid>> {
    let cluster_id = deployment_tasks::table
        .find(task_id)
        .select(deployment_tasks::cluster_id)
        .get_result::<Uuid>(db_conn().await?.deref_mut())
        .await
        .optional()?;
    match cluster_id {
        Some(cluster_id) => cluster_env_id(cluster_id).await,
        None => Ok(None),
    }
}

/// Resolve the environment of a deployment resource via its deployment.
async fn deployment_resource_env_id(resource_id: Uuid) -> DbResult<Option<Uuid>> {
    let deployment_id = deployment_resources::table
        .find(resource_id)
        .select(deployment_resources::deployment_id)
        .get_result::<Option<Uuid>>(db_conn().await?.deref_mut())
        .await
        .optional()?
        .flatten();
    match deployment_id {
        Some(deployment_id) => deployment_env_id(deployment_id).await,
        None => Ok(None),
    }
}

/// Resolve the (non-null) environment of a secret by primary key.
async fn secret_env_id(id: Uuid) -> DbResult<Option<Uuid>> {
    use crate::schema::secret::secrets;
    Ok(secrets::table
        .find(id)
        .select(secrets::env_id)
        .get_result::<Uuid>(db_conn().await?.deref_mut())
        .await
        .optional()?)
}

/// Resolve the (non-null) environment of an env-user permission by primary key.
async fn env_user_permission_env_id(id: Uuid) -> DbResult<Option<Uuid>> {
    Ok(env_user_permissions::table
        .find(id)
        .select(env_user_permissions::env_id)
        .get_result::<Uuid>(db_conn().await?.deref_mut())
        .await
        .optional()?)
}

/// Resolve the (non-null) environment of a deployment permission by primary key.
async fn deployment_permission_env_id(id: Uuid) -> DbResult<Option<Uuid>> {
    use crate::schema::deployment_permission::deployment_permissions;
    Ok(deployment_permissions::table
        .find(id)
        .select(deployment_permissions::env_id)
        .get_result::<Uuid>(db_conn().await?.deref_mut())
        .await
        .optional()?)
}
