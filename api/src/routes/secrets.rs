use crate::auth::CurUser;
use crate::permissions::verify_env_admin;
use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use platz_db::{DbTable, Deployment, NewSecret, Secret, UpdateSecret};
use uuid::Uuid;

#[actix_web::get("")]
async fn get_all() -> ApiResult {
    Ok(HttpResponse::Ok().json(Secret::all().await?))
}

#[actix_web::get("{id}")]
async fn get(id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(Secret::find(id.into_inner()).await?))
}

#[actix_web::post("")]
async fn create(cur_user: CurUser, new_secret: web::Json<NewSecret>) -> ApiResult {
    let new_secret = new_secret.into_inner();
    verify_env_admin(new_secret.env_id, cur_user.user().id).await?;
    Ok(HttpResponse::Created().json(new_secret.insert().await?))
}

#[actix_web::put("/{id}")]
async fn update(
    cur_user: CurUser,
    id: web::Path<Uuid>,
    update: web::Json<UpdateSecret>,
) -> ApiResult {
    let id = id.into_inner();
    let update = update.into_inner();

    let old = Secret::find(id).await?;
    verify_env_admin(old.env_id, cur_user.user().id).await?;
    let new = update.save(id).await?;

    Deployment::reinstall_all_using(
        DbTable::Secrets,
        id,
        cur_user.user(),
        format!("{} secret has been updated", new.collection),
    )
    .await?;

    Ok(HttpResponse::Ok().json(new))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/secrets")
            .service(get_all)
            .service(get)
            .service(create)
            .service(update),
    );
}
