use crate::result::ApiResult;
use actix_web::{get, HttpResponse};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct ServerInfo {
    pub version: String,
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Server",
    operation_id = "getServerInfo",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    responses(
        (
            status = OK,
            body = ServerInfo,
        ),
    ),
)]
#[get("/server")]
async fn get_one() -> ApiResult {
    Ok(HttpResponse::Ok().json(ServerInfo {
        version: std::env!("PLATZ_BACKEND_VERSION").to_string(),
    }))
}

#[derive(utoipa::OpenApi)]
#[openapi(
    tags((
        name = "Server",
        description = "Return information about the Platz server.",
    )),
    paths(get_one),
    components(schemas(ServerInfo)),
)]
pub(super) struct OpenApi;
