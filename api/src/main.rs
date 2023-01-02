use actix_web::middleware::Logger;
use actix_web::{error::InternalError, web, App, HttpResponse, HttpServer};
use anyhow::Result;
use clap::Parser;
use platz_db::init_db;
use serde_json::json;
use url::Url;

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

    /// Email addresses to add as admins instead of regular user. This option
    /// is useful for allowing the first admins to log into Platz on a fresh
    /// deployment. Note that admins are added only after successful validation
    /// against the OIDC server, and if a user doesn't exist with that email.
    /// This means that if an admin is later changed to a regular user role,
    /// they will never become an admin again unless their user is deleted from
    /// the database, or removed from this option.
    #[clap(long = "admin-email", env = "ADMIN_EMAILS", value_delimiter = ' ')]
    admin_emails: Vec<String>,
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
        platz_auth::OidcLogin::new(
            config.oidc_server_url,
            config.oidc_client_id,
            config.oidc_client_secret,
            config.admin_emails,
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
            .service(web::scope("/api/v2").configure(routes::v2::config))
    });

    Ok(server.bind(&format!("0.0.0.0:{}", api_port))?.run().await?)
}

async fn _main(config: Config) -> Result<()> {
    init_db(true).await?;
    serve(config).await
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::parse();
    env_logger::Builder::new()
        .filter(Some(env!("CARGO_PKG_NAME")), config.log_level())
        .filter(None, config.all_log_level())
        .init();

    _main(config).await
}
