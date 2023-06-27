use crate::permissions::verify_site_admin;
use crate::result::ApiResult;
use actix_web::{get, put, web, HttpResponse};
use platz_auth::ApiIdentity;
use platz_db::{DeploymentKind, DeploymentKindFilters, UpdateDeploymentKind};
use uuid::Uuid;

#[get("/deployment-kinds")]
async fn get_all(_identity: ApiIdentity, filters: web::Query<DeploymentKindFilters>) -> ApiResult {
    Ok(HttpResponse::Ok().json(DeploymentKind::all_filtered(filters.into_inner()).await?))
}

#[get("/deployment-kinds/{id}")]
async fn get_one(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(DeploymentKind::find(id.into_inner()).await?))
}

#[put("/deployment-kinds/{id}")]
async fn update(
    identity: ApiIdentity,
    id: web::Path<Uuid>,
    data: web::Json<UpdateDeploymentKind>,
) -> ApiResult {
    verify_site_admin(&identity).await?;
    let id = id.into_inner();
    let data = data.into_inner();
    Ok(HttpResponse::Ok().json(data.save(id).await?))
}
