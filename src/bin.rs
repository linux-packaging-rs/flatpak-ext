use std::{
    env,
    fs::remove_dir_all,
    path::{Path, PathBuf},
};

use clap::Parser;
use libflatrun::{Flatpak, FlatrunError, Message};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
/// Flatrun: Run Flatpak Applications without install!
struct Cli {
    /// Flatpak to run from file
    #[arg(short, long)]
    file: Option<String>,
    /// Dependency file (leave out to download dependencies automatically)
    #[arg(short, long)]
    dep: Option<String>,
    /// Flatpak appid to download
    #[arg(short, long)]
    app_id: Option<String>,
    /// Flatpak remote to use to download any flatpaks (defaults to flathub)
    #[arg(short, long)]
    remote: Option<String>,
    /// Clean out the temp repo directory
    #[arg(short, long)]
    clean: bool,
    /// Verbose
    #[arg(long)]
    verbose: bool,
}

fn main() -> Result<(), FlatrunError> {
    let cli = Cli::parse();
    if cli.verbose {
        // simple_logger::init_with_level(log::Level::Trace).unwrap();
    }
    log::info!("Starting flatrun!");

    if cli.clean {
        let _ = remove_dir_all(env::temp_dir().join("flatrun"));
        log::trace!("Cleared directory: {:?}", env::temp_dir().join("flatrun"));
    }
    for e in env::vars().map(|(x, y)| format!("{}={}", x, y)) {
        log::trace!("{e}");
    }

    match cli.file {
        Some(path) => {
            match libflatrun::run(
                libflatrun::Repo::temp(),
                Flatpak::Bundle(path_from_uri(path)),
                Some(libflatrun::Repo::default()),
                cli.dep.map(|x| Flatpak::Bundle(path_from_uri(x))),
                cli.remote,
                handle_message,
            ) {
                Ok(_) => Ok(()),
                Err(e) => {
                    log::error!("{:?}", e);
                    Err(e)
                }
            }
        }
        None => {
            if let Some(app_id) = cli.app_id {
                match libflatrun::run(
                    libflatrun::Repo::temp_in(env::temp_dir().join("flatrun")),
                    Flatpak::Download(app_id),
                    Some(libflatrun::Repo::default()),
                    cli.dep.map(|x| Flatpak::Bundle(path_from_uri(x))),
                    cli.remote,
                    handle_message,
                ) {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        log::error!("{:?}", e);
                        Err(e)
                    }
                }
            } else {
                if !cli.clean {
                    println!(
                        "'{} --help' to see args",
                        env::current_exe().unwrap().to_string_lossy()
                    );
                }
                Ok(())
            }
        }
    }
}

fn handle_message(msg: Message) {
    match msg {
        Message::Install { r, progress, .. } => {
            println!("Installing {}... {}%", r, progress * 100.0);
        }
        Message::Running { n } => {
            println!("Running {}", n);
        }
        Message::Unknown => {
            println!("Unknown message!");
        }
    }
}

fn path_from_uri(uri: String) -> PathBuf {
    if uri.starts_with("file://") {
        Path::new(uri.split_once("file://").unwrap().1).to_path_buf()
    } else {
        Path::new(&uri).to_path_buf()
    }
}
