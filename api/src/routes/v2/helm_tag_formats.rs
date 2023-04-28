use crate::permissions::verify_site_admin;
use crate::result::ApiResult;
use actix_web::{delete, get, post, web, HttpResponse};
use platz_auth::ApiIdentity;
use platz_db::{HelmTagFormat, HelmTagFormatFilters, NewHelmTagFormat};
use regex::Regex;
use serde_json::json;
use uuid::Uuid;

#[get("/helm-tag-formats")]
async fn get_all(_identity: ApiIdentity, filters: web::Query<HelmTagFormatFilters>) -> ApiResult {
    Ok(HttpResponse::Ok().json(HelmTagFormat::all_filtered(filters.into_inner()).await?))
}

#[get("/helm-tag-formats/{id}")]
async fn get_one(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(HelmTagFormat::find(id.into_inner()).await?))
}

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

#[delete("/helm-tag-formats/{id}")]
async fn delete(identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    let tag_format = HelmTagFormat::find(id.into_inner()).await?;
    verify_site_admin(&identity).await?;
    tag_format.delete().await?;
    Ok(HttpResponse::NoContent().finish())
}
