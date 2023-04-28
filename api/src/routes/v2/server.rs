use crate::result::ApiResult;
use actix_web::{get, HttpResponse};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ServerInfo {
    pub version: String,
}

#[get("/server")]
async fn get_one() -> ApiResult {
    Ok(HttpResponse::Ok().json(ServerInfo {
        version: std::env!("PLATZ_BACKEND_VERSION").to_string(),
    }))
}
