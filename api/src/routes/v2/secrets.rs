use super::deployments::using_error;
use crate::permissions::verify_env_admin;
use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use platz_auth::ApiIdentity;
use platz_db::{
    DbTable, DbTableOrDeploymentResource, Deployment, NewSecret, Secret, SecretFilters,
    UpdateSecret,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

async fn get_all(_identity: ApiIdentity, filters: web::Query<SecretFilters>) -> ApiResult {
    Ok(HttpResponse::Ok().json(Secret::all_filtered(filters.into_inner()).await?))
}

async fn get(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(Secret::find(id.into_inner()).await?))
}

async fn create(identity: ApiIdentity, new_secret: web::Json<NewSecret>) -> ApiResult {
    let new_secret = new_secret.into_inner();
    verify_env_admin(new_secret.env_id, &identity).await?;
    Ok(HttpResponse::Created().json(new_secret.insert().await?))
}

#[derive(Deserialize)]
struct UpdateSecretApi {
    name: Option<String>,
    contents: Option<String>,
}

impl From<UpdateSecretApi> for UpdateSecret {
    fn from(api: UpdateSecretApi) -> Self {
        Self::new(api.name, api.contents)
    }
}

async fn update(
    identity: ApiIdentity,
    id: web::Path<Uuid>,
    update: web::Json<UpdateSecretApi>,
) -> ApiResult {
    let id = id.into_inner();
    let update: UpdateSecret = update.into_inner().into();

    let old = Secret::find(id).await?;
    verify_env_admin(old.env_id, &identity).await?;
    let new = update.save(id).await?;

    Deployment::reinstall_all_using(
        &DbTableOrDeploymentResource::DbTable(DbTable::Secrets),
        id,
        &identity,
        format!("{} secret has been updated", new.collection),
    )
    .await?;

    Ok(HttpResponse::Ok().json(new))
}

async fn delete(identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    let id = id.into_inner();
    let secret = Secret::find(id).await?;

    verify_env_admin(secret.env_id, &identity).await?;

    let dependents =
        Deployment::find_using(&DbTableOrDeploymentResource::DbTable(DbTable::Secrets), id).await?;
    if !dependents.is_empty() {
        return Ok(HttpResponse::Conflict().json(json!({
            "message": using_error("This deployment can't be deleted because other deployments depend on it", dependents),
        })));
    }

    secret.delete().await?;
    Ok(HttpResponse::NoContent().finish())
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(get_all));
    cfg.route("/{id}", web::get().to(get));
    cfg.route("", web::post().to(create));
    cfg.route("/{id}", web::put().to(update));
    cfg.route("/{id}", web::delete().to(delete));
}
