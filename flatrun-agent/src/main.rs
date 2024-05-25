use std::{fmt::Display, io, path::PathBuf};

use clap::{Parser, Subcommand};

mod flatpak;

#[derive(Debug)]
enum FlatrunAgentError {
    Glib(libflatpak::glib::Error),
    IO(io::Error),
}

impl Display for FlatrunAgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Glib(e) => write!(f, "Glib error occurred: {}", e),
            Self::IO(e) => write!(f, "IO: {}", e),
        }
    }
}

impl From<libflatpak::glib::Error> for FlatrunAgentError {
    fn from(value: libflatpak::glib::Error) -> Self {
        Self::Glib(value)
    }
}

impl From<io::Error> for FlatrunAgentError {
    fn from(value: io::Error) -> Self {
        Self::IO(value)
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<AgentCommand>,
    #[arg(short, long)]
    remote: Option<String>,
}

#[derive(Subcommand, Clone, Debug)]
enum AgentCommand {
    InstallRunBundle {
        installation: PathBuf,
        deps_installation: PathBuf,
        path: PathBuf,
    },
    // InstallRunDownload {
    //     installation: PathBuf,
    //     deps_installation: PathBuf,
    //     appid: String,
    // },
}

fn main() -> Result<(), FlatrunAgentError> {
    let cli = Cli::parse();
    match cli.command {
        Some(AgentCommand::InstallRunBundle {
            installation,
            deps_installation,
            path,
        }) => match flatpak::install_bundle(installation, deps_installation, path) {
            Ok(_) => {}
            Err(e) => {
                log::error!("{:?}", e);
                eprintln!("{:?}", e);
            }
        },
        // Some(AgentCommand::InstallRunDownload {
        //     installation,
        //     deps_installation,
        //     appid,
        // }) => {}
        None => {}
    }
    Ok(())
}
