use crate::auth::{AccessToken, CurUser, OAuth2Response, OidcLogin};
use crate::result::ApiResult;
use actix_web::{dev::ConnectionInfo, web, HttpResponse};
use serde::Serialize;
use serde_json::json;
use url::Url;

fn callback_url(conn: &ConnectionInfo) -> Url {
    Url::parse(&format!(
        "{}://{}/auth/google/callback",
        conn.scheme(),
        conn.host()
    ))
    .expect("Failed creating callback URL")
}

#[actix_web::get("me")]
async fn me(cur_user: CurUser) -> ApiResult {
    Ok(HttpResponse::Ok().json(cur_user))
}

#[derive(Serialize)]
struct GoogleLoginInfo {
    redirect_url: Url,
}

#[actix_web::get("google")]
async fn google_login_info(conn: ConnectionInfo, oidc_login: web::Data<OidcLogin>) -> ApiResult {
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

#[actix_web::post("google/callback")]
async fn google_login_callback(
    req: web::HttpRequest,
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
    cfg.service(
        web::scope("/api/v1/auth")
            .service(me)
            .service(google_login_info)
            .service(google_login_callback),
    );
}
