mod flatpak;
mod remotes;

use std::{path::PathBuf, string::FromUtf8Error};

use clap::{arg, Parser, Subcommand};
use flatpak::DependencyInstall;

use crate::flatpak::{Flatpak, FlatpakRepo};

#[derive(Debug)]
pub enum FlatrunError {
    IO(std::io::Error),
    CommandUnsuccessful(String),
    FileNotFound(PathBuf),
    GLib(libflatpak::glib::Error),
}

impl From<std::io::Error> for FlatrunError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}

impl From<FromUtf8Error> for FlatrunError {
    fn from(e: FromUtf8Error) -> Self {
        Self::CommandUnsuccessful(e.to_string())
    }
}

impl From<libflatpak::glib::Error> for FlatrunError {
    fn from(value: libflatpak::glib::Error) -> Self {
        Self::GLib(value)
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    /// command to run
    command: Option<FlatrunCommand>,
}

#[derive(Subcommand)]
enum FlatrunCommand {
    /// run a flatpak file (.flatpak, .flatpakref)
    Run {
        #[command(subcommand)]
        reftype: RefType,
        /// run the flatpak offline (will error out if deps don't exist)
        #[arg(short, long)]
        offline: bool,
        /// put any non-downloaded dependencies in this repo (system|user|temp)
        #[arg(conflicts_with = "offline")]
        deps_to: Option<String>,
    },
    /// create a bundle for a flatpak
    CreateBundle {
        #[command(subcommand)]
        reftype: RefType,
        /// whether to include dependencies (i.e. runtimes) in the bundle
        #[arg(short, long)]
        include_deps: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
enum RefType {
    /// specify the flatpak by name and if not installed, a named remote
    Name {
        /// named remote for the flatpak (i.e. flathub)
        #[arg(value_name = "REMOTE")]
        remote: String,
        /// id of the flatpak (i.e. com.visualstudio.code)
        #[arg(value_name = "APPID")]
        app: String,
    },
    /// specify the flatpak by a path to a .flatpak or .flatpakref
    Path {
        /// path to the flatpak to run (.flatpak, .flatpakref)
        #[arg(value_name = "FILE")]
        path: PathBuf,
    },
}

fn main() -> Result<(), FlatrunError> {
    env_logger::init();
    let cli = Cli::parse();
    match &cli.command {
        Some(FlatrunCommand::Run {
            reftype,
            offline,
            deps_to,
        }) => {
            match run(
                reftype.clone(),
                *offline,
                DependencyInstall::from(deps_to.as_deref().unwrap_or_default()),
            ) {
                Ok(()) => Ok(()),
                Err(e) => {
                    log::error!("{:?}", e);
                    Ok(())
                }
            }
        }
        Some(FlatrunCommand::CreateBundle {
            reftype,
            include_deps,
        }) => match reftype {
            RefType::Name { remote, app } => Ok(()),
            RefType::Path { path } => Ok(()),
        },
        None => {
            log::error!("no option specified! use flatrun --help to see options");
            Ok(())
        }
    }
}

fn run(app: RefType, offline: bool, dependencies: DependencyInstall) -> Result<(), FlatrunError> {
    log::info!("requested flatpak: {:?}", app);
    let repo = FlatpakRepo::new(offline)?;
    let flatpak = Flatpak::new(app, &repo, dependencies, offline)?;
    flatpak.run(&repo)?;
    Ok(())
}
