use crate::{permissions::verify_site_admin, result::ApiResult};
use actix_web::{HttpResponse, delete, get, post, web};
use platz_auth::ApiIdentity;
use platz_db::{
    diesel_pagination::{Paginated, PaginationParams},
    schema::helm_tag_format::{HelmTagFormat, HelmTagFormatFilters, NewHelmTagFormat},
};
use regex::Regex;
use serde_json::json;
use uuid::Uuid;

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Helm Tag Formats",
    operation_id = "allHelmTagFormats",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    params(HelmTagFormatFilters),
    responses(
        (
            status = OK,
            body = Paginated<HelmTagFormat>,
        ),
    ),
)]
#[get("/helm-tag-formats")]
async fn get_all(
    _identity: ApiIdentity,
    filters: web::Query<HelmTagFormatFilters>,
    pagination: web::Query<PaginationParams>,
) -> ApiResult {
    Ok(HttpResponse::Ok()
        .json(HelmTagFormat::all_filtered(filters.into_inner(), pagination.into_inner()).await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Helm Tag Formats",
    operation_id = "getHelmTagFormat",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    responses(
        (
            status = OK,
            body = HelmTagFormat,
        ),
    ),
)]
#[get("/helm-tag-formats/{id}")]
async fn get_one(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(HelmTagFormat::find(id.into_inner()).await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Helm Tag Formats",
    operation_id = "createHelmTagFormat",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    request_body = NewHelmTagFormat,
    responses(
        (
            status = CREATED,
            body = HelmTagFormat,
        ),
    ),
)]
#[post("/helm-tag-formats")]
async fn create(identity: ApiIdentity, new_tag_format: web::Json<NewHelmTagFormat>) -> ApiResult {
    verify_site_admin(&identity).await?;
    let new_tag_format = new_tag_format.into_inner();
    Ok(if let Err(err) = Regex::new(&new_tag_format.pattern) {
        HttpResponse::BadRequest().json(json!({ "error": format!("Pattern error: {err}") }))
    } else {
        HttpResponse::Created().json(new_tag_format.insert().await?)
    })
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Helm Tag Formats",
    operation_id = "deleteHelmTagFormat",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    responses(
        (
            status = NO_CONTENT,
        ),
    ),
)]
#[delete("/helm-tag-formats/{id}")]
async fn delete(identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    let tag_format = HelmTagFormat::find(id.into_inner()).await?;
    verify_site_admin(&identity).await?;
    tag_format.delete().await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(utoipa::OpenApi)]
#[openapi(
    tags((
        name = "Helm Tag Formats",
        description = "\
Helm tag formats are how Platz parsed tags of Helm charts in registries.

Each format is a regular expression containing groups for the chart version,
Git commit and branch.
        ",
    )),
    paths(get_all, get_one, create, delete),
)]
pub(super) struct OpenApi;
