use crate::auth::{AccessToken, ApiIdentity, OAuth2Response, OidcLogin};
use crate::result::ApiResult;
use actix_web::{dev::ConnectionInfo, web, HttpRequest, HttpResponse};
use platz_db::{Deployment, Identity, User};
use serde::Serialize;
use serde_json::json;
use url::Url;
use uuid::Uuid;

fn callback_url(conn: &ConnectionInfo) -> Url {
    Url::parse(&format!(
        "{}://{}/auth/google/callback",
        conn.scheme(),
        conn.host()
    ))
    .expect("Failed creating callback URL")
}

#[derive(Serialize)]
enum MeResponse {
    User(User),
    Deployment { id: Uuid, name: String },
}

async fn me(identity: ApiIdentity) -> ApiResult {
    Ok(HttpResponse::Ok().json(match identity.into_inner() {
        Identity::User(user_id) => MeResponse::User(User::find(user_id).await?.unwrap()),
        Identity::Deployment(deployment_id) => {
            let Deployment { id, name, .. } = Deployment::find(deployment_id).await?;
            MeResponse::Deployment { id, name }
        }
    }))
}

#[derive(Serialize)]
struct GoogleLoginInfo {
    redirect_url: Url,
}

pub async fn google_login_info(
    conn: ConnectionInfo,
    oidc_login: web::Data<OidcLogin>,
) -> ApiResult {
    Ok(oidc_login
        .get_redirect_url(callback_url(&conn))
        .await
        .map(|redirect_url| HttpResponse::Ok().json(GoogleLoginInfo { redirect_url }))
        .unwrap_or_else(|e| {
            HttpResponse::InternalServerError().json(json!({
                "message": format!("Error getting redirect URL: {}", e),
            }))
        }))
}

pub async fn google_login_callback(
    req: HttpRequest,
    oidc_login: web::Data<OidcLogin>,
    oauth2_response: web::Json<OAuth2Response>,
) -> ApiResult {
    let user = oidc_login
        .login_user(
            callback_url(&req.connection_info()),
            oauth2_response.into_inner(),
        )
        .await?;

    let access_token: String = AccessToken::from(&user).encode().await?;
    Ok(HttpResponse::Ok().json(json!({
        "access_token": access_token,
    })))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("/me", web::get().to(me));
    cfg.route("/google", web::get().to(google_login_info));
    cfg.route("/google/callback", web::post().to(google_login_callback));
}
