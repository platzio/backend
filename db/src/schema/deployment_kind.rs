use crate::{DbResult, db_conn};
use chrono::prelude::*;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use diesel_filter::DieselFilter;
use diesel_pagination::{Paginate, Paginated, PaginationParams};
use serde::{Deserialize, Serialize};
use std::ops::DerefMut;
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
pub struct DeploymentKind {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    #[filter]
    pub name: String,
}

impl DeploymentKind {
    pub async fn all() -> DbResult<Vec<Self>> {
        Ok(deployment_kinds::table
            .get_results(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn all_filtered(
        filters: DeploymentKindFilters,
        pagination: PaginationParams,
    ) -> DbResult<Paginated<Self>> {
        Ok(Self::filter(filters)
            .paginate(pagination)
            .load_and_count(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn find(id: Uuid) -> DbResult<Self> {
        Ok(deployment_kinds::table
            .find(id)
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }

    pub async fn find_by_name(name: String) -> DbResult<Self> {
        Ok(deployment_kinds::table
            .filter(deployment_kinds::name.eq(name))
            .first(db_conn().await?.deref_mut())
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
            .get_result(db_conn().await?.deref_mut())
            .await?)
    }
}

#[derive(AsChangeset, Deserialize, ToSchema)]
#[diesel(table_name = deployment_kinds)]
pub struct UpdateDeploymentKind {
    pub name: String,
}

impl UpdateDeploymentKind {
    pub async fn save(self, id: Uuid) -> DbResult<DeploymentKind> {
        Ok(
            diesel::update(deployment_kinds::table.filter(deployment_kinds::id.eq(id)))
                .set(self)
                .get_result(db_conn().await?.deref_mut())
                .await?,
        )
    }
}
