use lazy_static::lazy_static;
use structopt::StructOpt;

lazy_static! {
    pub static ref CONFIG: Config = Default::default();
}

#[derive(StructOpt, Debug)]
pub struct Config {
    /// Turn debug logs on
    #[structopt(long)]
    debug: bool,

    /// Turn debug logs for all crates (not recommended)
    #[structopt(long)]
    all_debug: bool,

    #[structopt(long, env = "PLATZ_SELF_NAMESPACE")]
    self_namespace: String,

    #[structopt(long, env = "PLATZ_SELF_SERVICE_ACCOUNT_NAME")]
    self_service_account_name: String,

    #[structopt(long, env = "K8S_REFRESH_INTERVAL", default_value = "1h")]
    k8s_refresh_interval: humantime::Duration,

    #[structopt(long, env = "PLATZ_HELM_IMAGE")]
    helm_image: String,
}

impl Default for Config {
    fn default() -> Self {
        Self::from_args()
    }
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
}
