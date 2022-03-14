use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Config {
    /// Turn debug logs on
    #[structopt(long)]
    debug: bool,

    /// Turn debug logs for all crates (not recommended)
    #[structopt(long)]
    all_debug: bool,
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
}
