mod flatpak;

use std::{path::PathBuf, string::FromUtf8Error};

use clap::Parser;
use rustix::io::Errno;

use crate::flatpak::{Flatpak, FlatpakRepo};

#[derive(Debug)]
pub enum PortapakError {
    IO(std::io::Error),
    CommandUnsuccessful(String),
    FileNotFound(PathBuf),
    GLib(libflatpak::glib::Error),
    Errno(Errno),
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

impl From<libflatpak::glib::Error> for PortapakError {
    fn from(value: libflatpak::glib::Error) -> Self {
        Self::GLib(value)
    }
}

impl From<Errno> for PortapakError {
    fn from(value: Errno) -> Self {
        Self::Errno(value)
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
    let repo = FlatpakRepo::new()?;
    let flatpak = Flatpak::new(cli.app, &repo)?;
    flatpak.run(&repo)?;
    Ok(())
}
