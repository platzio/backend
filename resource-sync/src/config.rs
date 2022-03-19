use clap::Parser;

#[derive(Debug, Parser)]
pub struct Config {
    /// Turn debug logs on
    #[clap(long)]
    debug: bool,

    /// Turn debug logs for all crates (not recommended)
    #[clap(long)]
    all_debug: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self::parse()
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
