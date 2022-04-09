use crate::auth::CurIdentity;
use crate::permissions::verify_env_admin;
use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use platz_db::{
    DbTable, DbTableOrDeploymentResource, Deployment, NewSecret, Secret, SecretFilters,
    UpdateSecret,
};
use uuid::Uuid;

async fn get_all(_cur_identity: CurIdentity, filters: web::Query<SecretFilters>) -> ApiResult {
    Ok(HttpResponse::Ok().json(Secret::all_filtered(filters.into_inner()).await?))
}

async fn get(_cur_identity: CurIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(Secret::find(id.into_inner()).await?))
}

async fn create(cur_identity: CurIdentity, new_secret: web::Json<NewSecret>) -> ApiResult {
    let new_secret = new_secret.into_inner();
    verify_env_admin(new_secret.env_id, cur_identity.user().id).await?;
    Ok(HttpResponse::Created().json(new_secret.insert().await?))
}

async fn update(
    cur_identity: CurIdentity,
    id: web::Path<Uuid>,
    update: web::Json<UpdateSecret>,
) -> ApiResult {
    let id = id.into_inner();
    let update = update.into_inner();

    let old = Secret::find(id).await?;
    verify_env_admin(old.env_id, cur_identity.user().id).await?;
    let new = update.save(id).await?;

    Deployment::reinstall_all_using(
        &DbTableOrDeploymentResource::DbTable(DbTable::Secrets),
        id,
        cur_identity.user(),
        format!("{} secret has been updated", new.collection),
    )
    .await?;

    Ok(HttpResponse::Ok().json(new))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(get_all));
    cfg.route("/{id}", web::get().to(get));
    cfg.route("", web::post().to(create));
    cfg.route("/{id}", web::put().to(update));
}
