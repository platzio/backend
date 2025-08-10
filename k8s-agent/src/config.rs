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

    #[arg(long, default_value = "false")]
    pub disable_deployment_credentials: bool,

    #[arg(long, env = "PLATZ_OWN_URL")]
    pub platz_url: Url,
}

impl Config {
    pub fn should_refresh_deployment_credintials(&self) -> bool {
        !self.disable_deployment_credentials
    }
}
