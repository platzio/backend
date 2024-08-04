use crate::{pool, DbResult, Paginated, DEFAULT_PAGE_SIZE};
use async_diesel::*;
use chrono::prelude::*;
use diesel::prelude::*;
use diesel::QueryDsl;
use diesel_filter::{DieselFilter, Paginate};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

table! {
    deployment_kinds(id) {
        id -> Uuid,
        created_at -> Timestamptz,
        name -> Varchar,
    }
}

#[derive(Debug, Identifiable, Queryable, Serialize, DieselFilter, ToSchema)]
#[diesel(table_name = deployment_kinds)]
#[pagination]
pub struct DeploymentKind {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    #[filter]
    pub name: String,
}

impl DeploymentKind {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(deployment_kinds::table.get_results_async(pool()).await?)
    }

    pub async fn all_filtered(filters: DeploymentKindFilters) -> DbResult<Paginated<Self>> {
        let mut conn = pool().get()?;
        let page = filters.page.unwrap_or(1);
        let per_page = filters.per_page.unwrap_or(DEFAULT_PAGE_SIZE);
        let (items, num_total) = tokio::task::spawn_blocking(move || {
            Self::filter(&filters)
                .paginate(Some(page))
                .per_page(Some(per_page))
                .load_and_count::<Self>(&mut conn)
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
        Ok(deployment_kinds::table
            .find(id)
            .get_result_async(pool())
            .await?)
    }

    pub async fn find_by_name(name: String) -> DbResult<Self> {
        Ok(deployment_kinds::table
            .filter(deployment_kinds::name.eq(name))
            .first_async(pool())
            .await?)
    }
}

#[derive(Debug, Insertable, Deserialize, ToSchema)]
#[diesel(table_name = deployment_kinds)]
pub struct NewDeploymentKind {
    pub name: String,
}

impl NewDeploymentKind {
    pub async fn insert(self) -> DbResult<DeploymentKind> {
        Ok(diesel::insert_into(deployment_kinds::table)
            .values(self)
            .get_result_async(pool())
            .await?)
    }
}

#[derive(Debug, AsChangeset, Deserialize, ToSchema)]
#[diesel(table_name = deployment_kinds)]
pub struct UpdateDeploymentKind {
    pub name: String,
}

impl UpdateDeploymentKind {
    pub async fn save(self, id: Uuid) -> DbResult<DeploymentKind> {
        Ok(
            diesel::update(deployment_kinds::table.filter(deployment_kinds::id.eq(id)))
                .set(self)
                .get_result_async(pool())
                .await?,
        )
    }
}
