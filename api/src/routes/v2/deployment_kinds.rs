use crate::permissions::verify_site_admin;
use crate::result::ApiResult;
use actix_web::{get, put, web, HttpResponse};
use platz_auth::ApiIdentity;
use platz_db::{DeploymentKind, DeploymentKindFilters, Paginated, UpdateDeploymentKind};
use uuid::Uuid;

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Deployment Kinds",
    operation_id = "allDeploymentKinds",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    params(DeploymentKindFilters),
    responses(
        (
            status = OK,
            body = inline(Paginated<DeploymentKind>),
        ),
    ),
)]
#[get("/deployment-kinds")]
async fn get_all(_identity: ApiIdentity, filters: web::Query<DeploymentKindFilters>) -> ApiResult {
    Ok(HttpResponse::Ok().json(DeploymentKind::all_filtered(filters.into_inner()).await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Deployment Kinds",
    operation_id = "getDeploymentKind",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    responses(
        (
            status = OK,
            body = DeploymentKind,
        ),
    ),
)]
#[get("/deployment-kinds/{id}")]
async fn get_one(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(DeploymentKind::find(id.into_inner()).await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Deployment Kinds",
    operation_id = "updateDeploymentKind",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    request_body = UpdateDeploymentKind,
    responses(
        (
            status = OK,
            body = DeploymentKind,
        ),
    ),
)]
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

#[derive(utoipa::OpenApi)]
#[openapi(
    tags((
        name = "Deployment Kinds",
        description = "\
Deployment kinds map between kind IDs and their names.
        ",
    )),
    paths(get_all, get_one, update),
    components(schemas(
        DeploymentKind,
        UpdateDeploymentKind,
    )),
)]
pub(super) struct OpenApi;
