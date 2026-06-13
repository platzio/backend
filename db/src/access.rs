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
    schema::{env_user_permission::env_user_permissions, user::users},
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

    /// Decide whether the identity behind this scope may receive a websocket
    /// [`DbEvent`].
    ///
    /// The event's environment is carried on the event itself (resolved by the
    /// database trigger), so this is a cheap in-memory check with no query —
    /// and it works for `DELETE`s, where the row is already gone. Global tables
    /// that are not environment-scoped are visible to any authenticated
    /// identity; env-scoped events with no resolved environment fail closed.
    pub fn can_receive_event(&self, event: &DbEvent) -> bool {
        match self {
            Self::All => true,
            Self::Envs(env_ids) => match event.table {
                // Global catalog / infrastructure tables are not env-scoped and
                // are visible to every authenticated identity.
                DbTable::HelmTagFormats
                | DbTable::HelmCharts
                | DbTable::HelmRegistries
                | DbTable::DeploymentKinds
                | DbTable::DeploymentResourceTypes
                | DbTable::K8sClusters
                | DbTable::K8sResources
                | DbTable::Users
                | DbTable::Bots
                | DbTable::Settings => true,
                // Everything else is env-scoped: forward only when the event's
                // resolved environment is one the identity may access.
                _ => event.env_id.is_some_and(|env_id| env_ids.contains(&env_id)),
            },
        }
    }
}
