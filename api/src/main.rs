use actix_web::middleware::Logger;
use actix_web::{error::InternalError, web, App, HttpResponse, HttpServer};
use anyhow::Result;
use clap::Parser;
use log::*;
use platz_db::init_db;
use serde_json::json;
use url::Url;

mod auth;
mod permissions;
mod result;
mod routes;
mod serde_utils;

#[derive(Clone, Debug, Parser)]
struct Config {
    /// Turn debug logs on
    #[clap(long)]
    debug: bool,

    /// Turn debug logs for all crates (not recommended)
    #[clap(long)]
    all_debug: bool,

    #[clap(long, env = "API_PORT", default_value = "3000")]
    api_port: u16,

    #[clap(long, env = "OIDC_SERVER_URL")]
    oidc_server_url: Url,

    #[clap(long, env = "OIDC_CLIENT_ID")]
    oidc_client_id: String,

    #[clap(long, env = "OIDC_CLIENT_SECRET", hide_env_values = true)]
    oidc_client_secret: String,
}

impl Config {
    pub fn log_level(&self) -> log::LevelFilter {
        match self.debug {
            true => log::LevelFilter::Debug,
            false => log::LevelFilter::Info,
        }
    }

    pub fn all_log_level(&self) -> log::LevelFilter {
        match self.all_debug {
            true => log::LevelFilter::Debug,
            false => log::LevelFilter::Info,
        }
    }
}

async fn status() -> crate::result::ApiResult {
    Ok(HttpResponse::Ok().json("ok"))
}

async fn serve(config: Config) -> Result<()> {
    let api_port = config.api_port;
    let oidc_login = web::Data::new(
        crate::auth::OidcLogin::new(
            config.oidc_server_url,
            config.oidc_client_id,
            config.oidc_client_secret,
        )
        .await?,
    );

    let server = HttpServer::new(move || {
        let json_cfg = web::JsonConfig::default().error_handler(|err, _req| {
            let message = err.to_string();
            let res = HttpResponse::BadRequest().json(json!({
                "message": message,
            }));
            InternalError::from_response(err, res).into()
        });
        App::new()
            .wrap(Logger::default())
            .app_data(json_cfg)
            .app_data(oidc_login.clone())
            .route("/status", web::get().to(status))
            .service(web::scope("/api/v1").configure(routes::v1::config))
    });

    Ok(server.bind(&format!("0.0.0.0:{}", api_port))?.run().await?)
}

async fn _main(config: Config) -> Result<()> {
    init_db(true).await?;
    Ok(serve(config).await?)
}

#[actix_web::main]
async fn main() {
    let config = Config::parse();
    env_logger::Builder::new()
        .filter(Some(env!("CARGO_PKG_NAME")), config.log_level())
        .filter(None, config.all_log_level())
        .init();

    if let Err(e) = _main(config).await {
        error!("{:?}", e);
        std::process::exit(1);
    }
}
