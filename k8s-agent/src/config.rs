use url::Url;

#[derive(clap::Parser)]
#[group(skip)]
pub struct Config {
    #[command(flatten)]
    pub cluster_discovery: crate::k8s::cluster_discovery::Config,

    #[arg(long, env = "PLATZ_SELF_NAMESPACE")]
    pub self_namespace: String,

    #[arg(long, env = "PLATZ_SELF_SERVICE_ACCOUNT_NAME")]
    pub self_service_account_name: String,

    #[arg(long, env = "PLATZ_HELM_IMAGE")]
    pub helm_image: String,

    #[arg(
        long,
        env = "PLATZ_DISABLE_DEPLOYMENT_CREDENTIALS",
        default_value_t = false
    )]
    pub disable_deployment_credentials: bool,

    /// How often to refresh deployment credentials.
    #[arg(
        long,
        env = "PLATZ_DEPLOYMENT_CREDENTIALS_REFRESH_INTERVAL",
        default_value = "20m"
    )]
    pub deployment_credentials_refresh_interval: humantime::Duration,

    #[arg(long, env = "PLATZ_OWN_URL")]
    pub platz_url: Url,
}

impl Config {
    pub fn should_refresh_deployment_credintials(&self) -> bool {
        !self.disable_deployment_credentials
    }
}
