use clap::Parser;
use lazy_static::lazy_static;
use std::env;
use url::Url;

lazy_static! {
    pub static ref CONFIG: Config = Default::default();
    pub static ref OWN_URL: Url = Url::parse(
        &env::var("PLATZ_OWN_URL").expect("PLATZ_OWN_URL environment variable is not defined")
    )
    .unwrap();
}

#[derive(Debug, Parser)]
pub struct Config {
    #[clap(long, env = "PLATZ_SELF_NAMESPACE")]
    self_namespace: String,

    #[clap(long, env = "PLATZ_SELF_SERVICE_ACCOUNT_NAME")]
    self_service_account_name: String,

    #[clap(long, env = "K8S_REFRESH_INTERVAL", default_value = "1h")]
    k8s_refresh_interval: humantime::Duration,

    #[clap(long, env = "PLATZ_HELM_IMAGE")]
    helm_image: String,

    #[clap(long, default_value = "false")]
    disable_deployment_credentials: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self::parse()
    }
}

impl Config {
    pub fn self_namespace(&self) -> &str {
        &self.self_namespace
    }

    pub fn self_service_account_name(&self) -> &str {
        &self.self_service_account_name
    }

    pub fn k8s_refresh_interval(&self) -> core::time::Duration {
        self.k8s_refresh_interval.into()
    }

    pub fn helm_image(&self) -> &str {
        &self.helm_image
    }

    pub fn should_refresh_deployment_credintials(&self) -> bool {
        !self.disable_deployment_credentials
    }
}
