mod config;
mod run;

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use clap::Parser;
use ron::de::SpannedError;

use crate::{config::UserConfig, run::Flatpak};

#[derive(Debug)]
pub enum PortapakError {
    IO(std::io::Error),
    CommandUnsuccessful,
    FileNotFound(PathBuf),
    ConfigRead(SpannedError),
    ConfigWrite(ron::Error),
}

impl From<std::io::Error> for PortapakError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}

impl From<SpannedError> for PortapakError {
    fn from(value: SpannedError) -> Self {
        Self::ConfigRead(value)
    }
}

impl From<ron::Error> for PortapakError {
    fn from(value: ron::Error) -> Self {
        Self::ConfigWrite(value)
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Path to the .flatpak file to run
    #[arg(value_name = "FILE")]
    app: PathBuf,
}

fn main() -> Result<(), PortapakError> {
    env_logger::init();
    let cli = Cli::parse();
    if cli.config.clone().is_some_and(|x| !x.exists()) {
        return Err(PortapakError::FileNotFound(cli.config.unwrap()));
    }
    let conf_dir = cli.config.unwrap_or(
        Path::new(&format!(
            "{}/.config/portapak/config.ron",
            env::var("HOME").unwrap()
        ))
        .to_path_buf(),
    );
    if !conf_dir.exists() {
        fs::create_dir(conf_dir.parent().unwrap())?;
        fs::write(
            conf_dir.clone(),
            ron::ser::to_string_pretty(&UserConfig::default(), ron::ser::PrettyConfig::default())?,
        )?;
    }
    log::info!(
        "Config: {}, Flatpak: {}",
        conf_dir.as_os_str().to_string_lossy(),
        cli.app.as_os_str().to_string_lossy()
    );
    let config: UserConfig = ron::from_str(&fs::read_to_string(conf_dir)?)?;
    let flatpak = Flatpak::new(cli.app, config.clone())?;
    run::run_flatpak(flatpak, config)?;
    Ok(())
}
