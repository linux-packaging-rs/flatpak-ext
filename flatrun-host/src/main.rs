use std::{path::PathBuf, thread::sleep, time::Duration};

use clap::{Parser, Subcommand};
use libflatpak::{prelude::InstallationExtManual, prelude::InstanceExt, LaunchFlags};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    /// command to run
    command: FlatrunHostCommand,
}

#[derive(Subcommand)]
enum FlatrunHostCommand {
    Run {
        repo: PathBuf,
        appid: String,
        branch: String,
    },
}

#[derive(Debug, Clone)]
enum FlatrunHostError {
    GLib,
}

impl From<libflatpak::glib::Error> for FlatrunHostError {
    fn from(_: libflatpak::glib::Error) -> Self {
        Self::GLib
    }
}

fn main() -> Result<(), FlatrunHostError> {
    env_logger::init();
    let cli = Cli::parse();
    match cli.command {
        FlatrunHostCommand::Run {
            repo,
            appid,
            branch,
        } => match run(repo, appid, branch) {
            Ok(()) => Ok(()),
            Err(e) => {
                log::error!("flatrun-host: {:?}", e);
                Err(e)
            }
        },
    }
}

fn run(repo: PathBuf, appid: String, branch: String) -> Result<(), FlatrunHostError> {
    let repo_file = libflatpak::gio::File::for_path(repo);
    let installation = libflatpak::Installation::for_path(
        &repo_file,
        true,
        libflatpak::gio::Cancellable::current().as_ref(),
    )?;
    let inst = installation.launch_full(
        LaunchFlags::NONE,
        &appid,
        None,
        Some(&branch),
        None,
        libflatpak::gio::Cancellable::current().as_ref(),
    );
    match inst {
        Ok(i) => {
            while i.is_running() {
                sleep(Duration::from_millis(1000));
            }
            log::info!("Instance is no longer running! Exiting...");
            Ok(())
        }
        Err(e) => {
            log::error!("{}", e);
            Ok(())
        }
    }
}
