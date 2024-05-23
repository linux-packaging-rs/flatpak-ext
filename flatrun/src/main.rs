mod gui;
use std::{
    env, fs,
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
    process::Stdio,
};

use ashpd::desktop::file_chooser::{FileFilter, SelectedFiles};
use clap::{arg, Parser, Subcommand};
use flatpak_unsandbox::{Program, ProgramArg, UnsandboxError};
use iced::{
    futures::{
        channel::mpsc::{SendError, Sender},
        SinkExt,
    },
    Application, Settings,
};
use tempfile::TempDir;

use crate::gui::ProgressInfo;

#[derive(Debug)]
pub enum FlatrunError {
    IO(io::Error),
    Ashpd(ashpd::Error),
    Unsandbox(UnsandboxError),
    Iced(iced::Error),
    Send(SendError),
}

impl From<io::Error> for FlatrunError {
    fn from(value: io::Error) -> Self {
        Self::IO(value)
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

impl From<iced::Error> for FlatrunError {
    fn from(value: iced::Error) -> Self {
        Self::Iced(value)
    }
}

impl From<SendError> for FlatrunError {
    fn from(value: SendError) -> Self {
        Self::Send(value)
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
/// Flatrun: Run flatpak applications without install!
struct Cli {
    /// Run the graphical version of flatrun
    #[arg(short, long)]
    gui: bool,
    /// Optional url of remote to download from (defaults to Flathub)
    #[arg(short, long)]
    remote: Option<String>,
    #[command(subcommand)]
    command: Option<FlatrunCommand>,
}

#[derive(Subcommand)]
enum FlatrunCommand {
    /// Run a bundle saved locally
    Bundle {
        /// Path to the flatpak bundle to run (i.e. ~/inkscape.flatpak)
        #[arg(short, long)]
        bundle_path: String,
    },
    /// Download an app from a remote (Flathub by default)
    Download {
        /// App id for app to download (e.g. com.visualstudio.code)
        #[arg(short, long)]
        appid: String,
    },
}

#[async_std::main]
async fn main() -> Result<(), FlatrunError> {
    env_logger::init();
    let cli = Cli::parse();
    match cli.command {
        Some(FlatrunCommand::Bundle { bundle_path }) => {
            run_bundle(path_from_uri(bundle_path)?, cli.gui).await?;
            Ok(())
        }
        Some(FlatrunCommand::Download { appid }) => {
            let (temp_repo, deps_repo) = get_repos()?;
            log::info!(
                "temp_repo: {:?}, deps_repo: {:?}",
                temp_repo.path(),
                deps_repo
            );
            Ok(())
        }
        None => {
            let path = get_file_from_chooser().await?;
            run_bundle(path, cli.gui).await?;
            Ok(())
        }
    }
}

fn path_from_uri(path: String) -> Result<PathBuf, FlatrunError> {
    let pth = Path::new(&path.strip_prefix("file://").unwrap_or(&path)).to_owned();
    if !pth.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, pth.to_string_lossy()).into());
    }
    Ok(pth)
}

fn get_repos() -> Result<(TempDir, PathBuf), FlatrunError> {
    let temp_repo_parent = Path::new(
        &env::var("XDG_STATE_HOME")
            .unwrap_or(format!("{}/.cache/flatrun/", env::var("HOME").unwrap())),
    )
    .to_owned();
    let deps_repo = Path::new(&env::var("XDG_DATA_HOME").unwrap_or(format!(
        "{}/.local/share/flatrun",
        env::var("HOME").unwrap()
    )))
    .join("deps");
    let _ = fs::create_dir_all(&temp_repo_parent);
    let _ = fs::create_dir_all(&deps_repo);
    let temp_repo = TempDir::new_in(&temp_repo_parent)?;
    Ok((temp_repo, deps_repo))
}

async fn get_file_from_chooser() -> Result<PathBuf, FlatrunError> {
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
        Ok(Path::new(uri.path()).to_owned())
    } else {
        log::error!("no option specified! use flatrun --help to see options");
        Err(FlatrunError::IO(io::Error::new(
            io::ErrorKind::InvalidInput,
            "No option specified",
        )))
    }
}

pub async fn run_bundle(bundle_path: PathBuf, gui: bool) -> Result<(), FlatrunError> {
    if gui {
        let mut settings = Settings::with_flags(gui::RunApp::Bundle(bundle_path));
        settings.id = Some("io.github.ryanabx.flatrun".into());
        settings.window.platform_specific.application_id = "io.github.ryanabx.flatrun".into();
        ProgressInfo::run(settings)?;
        Ok(())
    } else {
        run_bundle_inner(bundle_path, &mut None).await?;
        Ok(())
    }
}

pub async fn run_bundle_inner(
    bundle_path: PathBuf,
    sender: &mut Option<&mut Sender<gui::Message>>,
) -> Result<(), FlatrunError> {
    let (temp_repo, deps_repo) = get_repos()?;
    log::info!(
        "temp_repo: {:?}, deps_repo: {:?}",
        temp_repo.path(),
        deps_repo
    );
    println!("Unsandboxing...");
    if let Some(mut cmd) = flatpak_unsandbox::unsandbox(Some(Program::new(
        "/app/libexec/flatrun-agent",
        Some(vec![
            ProgramArg::Value("install-run-bundle".into()), // command
            ProgramArg::Path {
                path: temp_repo.path().to_path_buf(),
                in_sandbox: false, // '/tmp' is not in the sandbox
            }, // repo
            ProgramArg::Path {
                path: deps_repo,
                in_sandbox: true,
            }, // dependency repo
            ProgramArg::Path {
                path: bundle_path,
                in_sandbox: false,
            }, // bundle
        ]),
        Some(vec![("RUST_LOG".into(), "DEBUG".into())]), // logging
    )))? {
        println!("Unsandboxin2g...");
        let mut child = cmd.stdout(Stdio::piped()).spawn().unwrap();
        let stdout = child.stdout.take().unwrap();
        // Stream output.
        let lines = BufReader::new(stdout).lines();
        for line in lines {
            let l = line.unwrap();
            let update_metadata = l.split("::").map(|x| x.to_string()).collect::<Vec<_>>();
            println!("GOT LINE: {:?}", update_metadata);
            if update_metadata.len() != 5 {
                if l.contains("RUNNING_APPLICATION") {
                    if let Some(s) = sender {
                        log::info!("Sending hide command!");
                        s.send(gui::Message::Hide).await?;
                    }
                }
                continue;
            }
            let repo = update_metadata.get(0).unwrap().clone();
            let action = update_metadata.get(1).unwrap().clone();
            let app_ref = update_metadata.get(2).unwrap().clone();
            let message = update_metadata.get(3).unwrap().clone();
            let progress = update_metadata
                .get(4)
                .unwrap()
                .clone()
                .parse::<i32>()
                .unwrap();
            if let Some(s) = sender {
                log::info!("Sending the info!");
                s.send(gui::Message::UpdateProgress((
                    repo,
                    action,
                    app_ref,
                    message,
                    progress as f32 / 100.0,
                )))
                .await?;
            }
        }
    }
    log::info!("Cleaning up temp repo: {:?}", temp_repo.path());
    Ok(())
}
