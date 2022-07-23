use crate::{pool, DbError, DbResult, Paginated, DEFAULT_PAGE_SIZE};
use async_diesel::*;
use chrono::prelude::*;
use diesel::prelude::*;
use diesel::QueryDsl;
use diesel_filter::{DieselFilter, Paginate};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

table! {
    helm_registries(id) {
        id -> Uuid,
        created_at -> Timestamptz,
        domain_name -> Varchar,
        repo_name -> Varchar,
        kind -> Varchar,
        available -> Bool,
        fa_icon -> Varchar,
    }
}

#[derive(Debug, Identifiable, Queryable, Serialize, DieselFilter)]
#[table_name = "helm_registries"]
#[pagination]
pub struct HelmRegistry {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub domain_name: String,
    #[filter]
    pub repo_name: String,
    #[filter]
    pub kind: String,
    pub available: bool,
    pub fa_icon: String,
}

impl HelmRegistry {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(helm_registries::table.get_results_async(pool()).await?)
    }

    pub async fn all_filtered(filters: HelmRegistryFilters) -> DbResult<Paginated<Self>> {
        let conn = pool().get()?;
        let page = filters.page.unwrap_or(1);
        let per_page = filters.per_page.unwrap_or(DEFAULT_PAGE_SIZE);
        let (items, num_total) = tokio::task::spawn_blocking(move || {
            Self::filter(&filters)
                .paginate(Some(page))
                .per_page(Some(per_page))
                .load_and_count::<Self>(&conn)
        })
        .await
        .unwrap()?;
        Ok(Paginated {
            page,
            per_page,
            num_total,
            items,
        })
    }

    pub async fn find(id: Uuid) -> DbResult<Self> {
        Ok(helm_registries::table
            .find(id)
            .get_result_async(pool())
            .await?)
    }

    pub async fn find_by_domain_and_repo(
        domain_name: String,
        repo_name: String,
    ) -> DbResult<Option<Self>> {
        Ok(helm_registries::table
            .filter(helm_registries::domain_name.eq(domain_name))
            .filter(helm_registries::repo_name.eq(repo_name))
            .first_async(pool())
            .await
            .optional()?)
    }

    pub fn region_name(&self) -> DbResult<String> {
        let (_aws_account_id, _dkr, _ecr, region_name, _amazonaws, _com) = self
            .domain_name
            .split('.')
            .collect_tuple()
            .ok_or(DbError::RegionNameParseError)?;
        Ok(region_name.into())
    }
}

#[derive(Debug, Insertable, Deserialize)]
#[table_name = "helm_registries"]
pub struct NewHelmRegistry {
    pub created_at: DateTime<Utc>,
    pub domain_name: String,
    pub repo_name: String,
    pub kind: String,
}

impl NewHelmRegistry {
    pub async fn insert(self) -> DbResult<HelmRegistry> {
        let domain_name = self.domain_name.clone();
        let repo_name = self.repo_name.clone();
        Ok(match diesel::insert_into(helm_registries::table)
           .values(self)
           .on_conflict_do_nothing()
           .get_result_async(pool())
           .await
           .optional()? {
               Some(registry) => registry,
               None => HelmRegistry::find_by_domain_and_repo(domain_name, repo_name).await?.expect(
                   "HelmRegistry::find_by_domain_and_repo returned empty result after successful NewHelmRegistry::insert"),
           })
    }
}

#[derive(Debug, AsChangeset, Deserialize)]
#[table_name = "helm_registries"]
pub struct UpdateHelmRegistry {
    pub fa_icon: Option<String>,
}

impl UpdateHelmRegistry {
    pub async fn save(self, id: Uuid) -> DbResult<HelmRegistry> {
        Ok(
            diesel::update(helm_registries::table.filter(helm_registries::id.eq(id)))
                .set(self)
                .get_result_async(pool())
                .await?,
        )
    }
}

#[derive(Debug, AsChangeset, Deserialize)]
#[table_name = "helm_registries"]
pub struct UpdateHelmRegistryKind {
    pub kind: String,
}

impl UpdateHelmRegistryKind {
    pub async fn save(self, id: Uuid) -> DbResult<HelmRegistry> {
        Ok(
            diesel::update(helm_registries::table.filter(helm_registries::id.eq(id)))
                .set(self)
                .get_result_async(pool())
                .await?,
        )
    }
}
