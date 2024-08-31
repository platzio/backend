use crate::result::ApiResult;
use actix_web::{web, HttpResponse};
use platz_auth::ApiIdentity;
use platz_db::{Identity, User};
use serde_json::json;

async fn me(identity: ApiIdentity) -> ApiResult {
    match identity.into_inner() {
        Identity::User(user_id) => Ok(HttpResponse::Ok().json(User::find(user_id).await?.unwrap())),
        Identity::Bot(_) => Ok(HttpResponse::BadRequest().json(json!({
            "message": "API v1 doesn't support bot authentication, please switch to v2"
        }))),
        Identity::Deployment(_) => Ok(HttpResponse::BadRequest().json(json!({
            "message": "API v1 doesn't support deployment authentication, please switch to v2"
        }))),
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("/me", web::get().to(me));
}
