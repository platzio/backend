use crate::{DbError, DbResult, db_conn};
use chrono::prelude::*;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use diesel_enum_derive::DieselEnum;
use diesel_filter::DieselFilter;
use diesel_pagination::{Paginate, Paginated, PaginationParams};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::ops::DerefMut;
use strum::{Display, EnumString};
use utoipa::ToSchema;
use uuid::Uuid;

table! {
    helm_registries(id) {
        id -> Uuid,
        created_at -> Timestamptz,
        domain_name -> Varchar,
        repo_name -> Varchar,
        kind_id -> Uuid,
        available -> Bool,
        fa_icon -> Varchar,
        provider -> Varchar,
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    EnumString,
    Display,
    DieselEnum,
    ToSchema,
)]
pub enum HelmRegistryProvider {
    /// AWS Elastic Container Registry. Authenticated with `aws ecr get-login-password`.
    #[default]
    Ecr,
    /// Generic OCI registry (e.g. Docker Distribution `registry:2`, zot, ghcr.io).
    /// No authentication is performed by Platz; the registry is expected to be
    /// anonymous-readable from the cluster running the Helm pod.
    Oci,
}

#[derive(Debug, Identifiable, Queryable, Serialize, DieselFilter, ToSchema)]
#[diesel(table_name = helm_registries)]
pub struct HelmRegistry {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub domain_name: String,
    #[filter]
    pub repo_name: String,
    #[filter]
    pub kind_id: Uuid,
    pub available: bool,
    pub fa_icon: String,
    pub provider: HelmRegistryProvider,
}

impl HelmRegistry {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(helm_registries::table
            .get_results(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn all_filtered(
        filters: HelmRegistryFilters,
        pagination: PaginationParams,
    ) -> DbResult<Paginated<Self>> {
        Ok(Self::filter(filters)
            .paginate(pagination)
            .load_and_count(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn find(id: Uuid) -> DbResult<Self> {
        Ok(helm_registries::table
            .find(id)
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn find_by_domain_and_repo(
        domain_name: String,
        repo_name: String,
    ) -> DbResult<Option<Self>> {
        Ok(helm_registries::table
            .filter(helm_registries::domain_name.eq(domain_name))
            .filter(helm_registries::repo_name.eq(repo_name))
            .first(db_conn().await?.deref_mut())
            .await
            .optional()?)
    }

    /// Returns the AWS region embedded in the registry's domain name, when applicable.
    /// Returns `Ok(None)` for non-ECR providers, since the concept doesn't apply.
    pub fn region_name(&self) -> DbResult<Option<String>> {
        match self.provider {
            HelmRegistryProvider::Oci => Ok(None),
            HelmRegistryProvider::Ecr => {
                let (_aws_account_id, _dkr, _ecr, region_name, _amazonaws, _com) = self
                    .domain_name
                    .split('.')
                    .collect_tuple()
                    .ok_or(DbError::RegionNameParseError)?;
                Ok(Some(region_name.into()))
            }
        }
    }
}

#[derive(Debug, Insertable, Deserialize, ToSchema)]
#[diesel(table_name = helm_registries)]
pub struct NewHelmRegistry {
    pub created_at: DateTime<Utc>,
    pub domain_name: String,
    pub repo_name: String,
    pub kind_id: Uuid,
    #[serde(default)]
    pub provider: HelmRegistryProvider,
}

impl NewHelmRegistry {
    pub async fn insert(self) -> DbResult<HelmRegistry> {
        let domain_name = self.domain_name.clone();
        let repo_name = self.repo_name.clone();
        Ok(match diesel::insert_into(helm_registries::table)
           .values(self)
           .on_conflict_do_nothing()
           .get_result(db_conn().await?.deref_mut())
           .await
           .optional()? {
               Some(registry) => registry,
               None => HelmRegistry::find_by_domain_and_repo( domain_name, repo_name).await?.expect(
                   "HelmRegistry::find_by_domain_and_repo returned empty result after successful NewHelmRegistry::insert"),
           })
    }
}

#[derive(Debug, AsChangeset, Deserialize, ToSchema)]
#[diesel(table_name = helm_registries)]
pub struct UpdateHelmRegistry {
    pub fa_icon: Option<String>,
}

impl UpdateHelmRegistry {
    pub async fn save(self, id: Uuid) -> DbResult<HelmRegistry> {
        Ok(
            diesel::update(helm_registries::table.filter(helm_registries::id.eq(id)))
                .set(self)
                .get_result(db_conn().await?.deref_mut())
                .await?,
        )
    }
}

#[derive(Debug, AsChangeset, Deserialize, ToSchema)]
#[diesel(table_name = helm_registries)]
pub struct UpdateHelmRegistryKind {
    pub kind_id: Uuid,
}

impl UpdateHelmRegistryKind {
    pub async fn save(self, id: Uuid) -> DbResult<HelmRegistry> {
        Ok(
            diesel::update(helm_registries::table.filter(helm_registries::id.eq(id)))
                .set(self)
                .get_result(db_conn().await?.deref_mut())
                .await?,
        )
    }
}
