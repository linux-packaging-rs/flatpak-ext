mod flatpak;

use std::{path::PathBuf, string::FromUtf8Error};

use clap::Parser;

use crate::flatpak::Flatpak;

#[derive(Debug)]
pub enum PortapakError {
    IO(std::io::Error),
    CommandUnsuccessful(String),
    FileNotFound(PathBuf),
}

impl From<std::io::Error> for PortapakError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}

impl From<FromUtf8Error> for PortapakError {
    fn from(e: FromUtf8Error) -> Self {
        Self::CommandUnsuccessful(e.to_string())
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to the .flatpak file to run
    #[arg(value_name = "FILE")]
    app: PathBuf,
}

fn main() -> Result<(), PortapakError> {
    env_logger::init();
    let cli = Cli::parse();
    log::info!(
        "requested flatpak: {}",
        cli.app.as_os_str().to_string_lossy()
    );
    let flatpak = Flatpak::new(cli.app)?;
    flatpak.run()?;
    Ok(())
}
