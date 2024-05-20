mod flatpak;

use std::{
    path::{Path, PathBuf},
    string::FromUtf8Error,
};

use ashpd::desktop::file_chooser::{FileFilter, SelectedFiles};
use async_std::task::spawn;
use clap::{arg, Args, Parser};
use flatpak::DependencyInstall;
use flatpak_unsandbox::UnsandboxError;
use slint::Weak;

use crate::flatpak::{Flatpak, FlatpakRepo};

#[derive(Debug)]
pub enum FlatrunError {
    IO(std::io::Error),
    CommandUnsuccessful(String),
    FileNotFound(PathBuf),
    GLib(libflatpak::glib::Error),
    Ashpd(ashpd::Error),
    Unsandbox(UnsandboxError),
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

impl From<ashpd::Error> for FlatrunError {
    fn from(value: ashpd::Error) -> Self {
        Self::Ashpd(value)
    }
}

impl From<UnsandboxError> for FlatrunError {
    fn from(value: UnsandboxError) -> Self {
        Self::Unsandbox(value)
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
/// Flatrun: Run flatpak applications without install!
struct Cli {
    #[arg(short, long)]
    /// Run the graphical version of flatrun
    gui: bool,
    #[arg(short, long, conflicts_with = "from_download")]
    #[arg(short, long, conflicts_with = "path", requires_all = ["remote", "name"])]
    /// Download a flatpak from a named remote
    download: bool,
    #[arg(
        short,
        long,
        value_name = "DOWNLOAD:REMOTE",
        conflicts_with = "path",
        requires_all = ["download", "name"]
    )]
    /// Named remote for the flatpak (i.e. flathub)
    remote: Option<String>,
    #[arg(
        short,
        long,
        value_name = "DOWNLOAD:APPID",
        conflicts_with = "path",
        requires_all = ["download", "remote"]
    )]
    /// Application id of the flatpak (i.e. com.visualstudio.code)
    name: Option<String>,
    #[arg(
        value_name = "BY_PATH:PATH",
        conflicts_with_all = ["download", "remote", "name"]
    )]
    /// Path of the .flatpak file to run (i.e. /home/1000/inkscape.flatpak)
    path: Option<String>,
}

#[derive(Args, Debug, Clone)]
struct NameCommand {
    /// named remote for the flatpak (i.e. flathub)
    #[arg(value_name = "REMOTE", conflicts_with = "path")]
    remote: String,
    /// id of the flatpak (i.e. com.visualstudio.code)
    #[arg(value_name = "APPID", conflicts_with = "path")]
    app: String,
}

#[derive(Clone, Debug)]
enum RefType {
    Path(PathBuf),
    Name { remote: String, app: String },
}

slint::include_modules!();

#[async_std::main]
async fn main() -> Result<(), FlatrunError> {
    env_logger::init();
    let cli = Cli::parse();
    log::debug!("Running unsandbox");
    if flatpak_unsandbox::unsandbox(None)? {
        return Ok(());
    }
    if cli.gui {
        if let Ok(ref_type) = get_file_from_chooser().await {
            let main_window = MainWindow::new().unwrap();
            let ui_handle = main_window.as_weak();
            let thread_handle = spawn(async move {
                if let Err(e) = run(
                    ref_type.clone(),
                    DependencyInstall::default(),
                    Some(&ui_handle),
                ) {
                    log::error!("{:?}", e);
                }
            });
            main_window.run().unwrap();
            thread_handle.await;
        }
        Ok(())
    } else {
        let ref_type = if !cli.download {
            if let Some(path) = cli.path {
                let pth = Path::new(&path.strip_prefix("file://").unwrap_or(&path)).to_owned();
                if !pth.exists() {
                    return Err(FlatrunError::FileNotFound(pth));
                }
                RefType::Path(pth)
            } else {
                get_file_from_chooser().await?
            }
        } else {
            if let (Some(remote), Some(app)) = (cli.remote, cli.name) {
                RefType::Name { remote, app }
            } else {
                return Err(FlatrunError::CommandUnsuccessful(
                    "[DOWNLOAD] You must specify a remote and an appid.".to_string(),
                ));
            }
        };
        match run(ref_type.clone(), DependencyInstall::default(), None) {
            Ok(()) => Ok(()),
            Err(e) => {
                log::error!("flatrun: {:?}", e);
                Ok(())
            }
        }
    }
}

fn run(
    app: RefType,
    dependencies: DependencyInstall,
    window_handle: Option<&Weak<MainWindow>>,
) -> Result<(), FlatrunError> {
    log::info!("requested flatpak: {:?}", app);
    let repo = FlatpakRepo::new()?;
    let flatpak = Flatpak::new(app, &repo, dependencies, window_handle)?;
    flatpak.run(&repo)?;
    Ok(())
}

async fn get_file_from_chooser() -> Result<RefType, FlatrunError> {
    // use xdg-desktop-portal
    let files = SelectedFiles::open_file()
        .title("Flatrun: Choose a flatpak to run!")
        .accept_label("Run Flatpak")
        .modal(true)
        .multiple(false)
        .filter(FileFilter::new(".flatpak").mimetype("application/vnd.flatpak"))
        .send()
        .await?
        .response()?;
    if let Some(uri) = files.uris().first() {
        println!("Got path {}", uri.path());
        Ok(RefType::Path(Path::new(uri.path()).to_owned()))
    } else {
        log::error!("no option specified! use flatrun --help to see options");
        Err(FlatrunError::CommandUnsuccessful(
            "[BY_PATH] No file specified".to_string(),
        ))
    }
}
