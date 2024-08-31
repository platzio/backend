use crate::result::ApiResult;
use actix_web::{get, post, web, HttpResponse};
use platz_auth::{AccessToken, ApiIdentity, OAuth2Response, OidcLogin};
use platz_db::{Bot, Deployment, Identity, User};
use serde::Serialize;
use serde_json::json;
use std::env;
use url::Url;
use utoipa::ToSchema;
use uuid::Uuid;

lazy_static::lazy_static! {
    static ref OWN_URL: Url = Url::parse(
        &env::var("PLATZ_OWN_URL").expect("PLATZ_OWN_URL environment variable is not defined")
    )
    .expect("Failed parsing PLATZ_OWN_URL, please verify it is a valid URL");

    static ref CALLBACK_URL: Url = OWN_URL
        .join("/auth/google/callback")
        .expect("Failed creating callback URL");
}

#[derive(Serialize, ToSchema)]
enum MeResponse {
    User(User),
    Bot(Bot),
    Deployment { id: Uuid, name: String },
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Authentication",
    operation_id = "authMe",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    responses(
        (
            status = OK,
            body = MeResponse,
        ),
    ),
)]
#[get("/auth/me")]
async fn me(identity: ApiIdentity) -> ApiResult {
    Ok(HttpResponse::Ok().json(match identity.into_inner() {
        Identity::User(user_id) => MeResponse::User(
            User::find(user_id)
                .await?
                .expect("User is authenticated but not found in database, this should not happen"),
        ),
        Identity::Bot(bot_id) => MeResponse::Bot(
            Bot::find(bot_id)
                .await?
                .expect("Bot is authenticated but not found in database, this should not happen"),
        ),
        Identity::Deployment(deployment_id) => {
            let Deployment { id, name, .. } = Deployment::find(deployment_id).await?;
            MeResponse::Deployment { id, name }
        }
    }))
}

#[derive(Serialize, ToSchema)]
struct StartGoogleLoginResponse {
    #[schema(value_type = String)]
    redirect_url: Url,
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Authentication",
    operation_id = "startGoogleLogin",
    responses(
        (
            status = OK,
            body = StartGoogleLoginResponse,
        ),
    ),
)]
#[get("/auth/google")]
pub async fn start_google_login(oidc_login: web::Data<OidcLogin>) -> ApiResult {
    Ok(oidc_login
        .get_redirect_url(&CALLBACK_URL)
        .await
        .map(|redirect_url| HttpResponse::Ok().json(StartGoogleLoginResponse { redirect_url }))
        .unwrap_or_else(|e| {
            HttpResponse::InternalServerError().json(json!({
                "message": format!("Error getting redirect URL: {e}"),
            }))
        }))
}

#[derive(Serialize, ToSchema)]
struct FinishGoogleLoginResponse {
    access_token: String,
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Authentication",
    operation_id = "finishGoogleLogin",
    request_body = OAuth2Response,
    responses(
        (
            status = OK,
            body = FinishGoogleLoginResponse,
        ),
    ),
)]
#[post("/auth/google/callback")]
pub async fn finish_google_login(
    oidc_login: web::Data<OidcLogin>,
    oauth2_response: web::Json<OAuth2Response>,
) -> ApiResult {
    let user = oidc_login
        .login_user(&CALLBACK_URL, oauth2_response.into_inner())
        .await?;

    let access_token: String = AccessToken::from(&user).encode().await?;
    Ok(HttpResponse::Ok().json(FinishGoogleLoginResponse { access_token }))
}

#[derive(utoipa::OpenApi)]
#[openapi(
    tags((
        name = "Authentication",
        description = "\
APIs for logging into Platz and getting information about the current user.
",
    )),
    paths(me, start_google_login, finish_google_login),
    components(schemas(
        MeResponse,
        User,
        StartGoogleLoginResponse,
        OAuth2Response,
        FinishGoogleLoginResponse,
    ))
)]
pub(super) struct OpenApi;
