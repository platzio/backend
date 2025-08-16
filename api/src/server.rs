use actix_web::middleware::Logger;
use actix_web::{App, HttpResponse, HttpServer, error::InternalError, web};
use anyhow::Result;
use prometheus::Encoder;
use serde_json::json;

#[derive(clap::Args)]
#[group(skip)]
pub struct Config {
    #[arg(long, env = "API_HOST", default_value = "0.0.0.0")]
    api_host: String,
    #[arg(long, env = "API_PORT", default_value = "3000")]
    api_port: u16,
    #[clap(flatten)]
    auth_config: platz_auth::Config,
}

pub async fn serve(config: Config) -> Result<()> {
    let Config {
        api_host,
        api_port,
        auth_config,
    } = config;
    let oidc_login = web::Data::new(platz_auth::OidcLogin::from(auth_config));

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
            .route("/metrics", web::get().to(metrics))
            .service(web::scope("/api/v2").configure(crate::routes::v2::config))
    });

    Ok(server
        .bind(&format!("{api_host}:{api_port}"))?
        .run()
        .await?)
}

async fn status() -> crate::result::ApiResult {
    Ok(HttpResponse::Ok().json("ok"))
}

async fn metrics() -> HttpResponse {
    let encoder = prometheus::TextEncoder::new();
    let mut buffer = Vec::new();
    let metric_families = prometheus::gather();
    // Encode them to send.
    encoder.encode(&metric_families, &mut buffer).unwrap();
    match String::from_utf8(buffer) {
        Ok(body) => HttpResponse::Ok().body(body),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}
