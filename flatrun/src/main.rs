mod gui;
use std::{
    env, fs,
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
    process::Stdio,
};

use ashpd::{
    desktop::file_chooser::{FileFilter, SelectedFiles},
    WindowIdentifier,
};
use clap::{arg, Parser, Subcommand};
use gui::AppState;
use iced::{
    futures::{
        channel::mpsc::{SendError, Sender},
        SinkExt,
    },
    window::Position,
    Application, Settings,
};
use rand::{distributions::Alphanumeric, Rng};
use rustix::thread::Pid;

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
    /// Clean out the temp repo directory
    #[arg(short, long)]
    clean: bool,
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
    // /// Download an app from a remote (Flathub by default)
    // Download {
    //     /// App id for app to download (e.g. com.visualstudio.code)
    //     #[arg(short, long)]
    //     appid: String,
    // },
}

#[async_std::main]
async fn main() -> Result<(), FlatrunError> {
    env_logger::init();
    let cli = Cli::parse();
    if cli.clean {
        clean_repos();
    }
    match cli.command {
        Some(FlatrunCommand::Bundle { bundle_path }) => {
            run_bundle(path_from_uri(bundle_path)?, cli.gui).await?;
            Ok(())
        }
        // Some(FlatrunCommand::Download { appid }) => {
        //     let (temp_repo, deps_repo) = get_repos()?;
        //     log::info!("temp_repo: {:?}, deps_repo: {:?}", temp_repo, deps_repo);
        //     Ok(())
        // }
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

fn get_repos() -> Result<(PathBuf, PathBuf), FlatrunError> {
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
    let foldername: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(7)
        .map(char::from)
        .collect();
    let temp_repo = temp_repo_parent.join(format!(".tmp{}", foldername));
    Ok((temp_repo, deps_repo))
}

fn clean_repos() {
    let temp_repo_parent = Path::new(
        &env::var("XDG_STATE_HOME")
            .unwrap_or(format!("{}/.cache/flatrun/", env::var("HOME").unwrap())),
    )
    .to_owned();
    let _ = std::fs::remove_dir_all(&temp_repo_parent);
    let _ = std::fs::create_dir(&temp_repo_parent);
    log::info!("Cleaned repos!")
}

async fn get_file_from_chooser() -> Result<PathBuf, FlatrunError> {
    // use xdg-desktop-portal
    let files = SelectedFiles::open_file()
        .title("Flatrun: Choose a flatpak to run!")
        .accept_label("Run Flatpak")
        .modal(true)
        .multiple(false)
        .identifier(WindowIdentifier::default())
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
    let (temp_repo, deps_repo) = get_repos()?;
    if gui {
        let mut settings = Settings::with_flags((
            temp_repo.clone(),
            deps_repo,
            AppState::LoadingFile(gui::RunApp::Bundle(bundle_path)),
        ));
        settings.id = Some("io.github.ryanabx.flatrun".into());
        settings.window.platform_specific.application_id = "io.github.ryanabx.flatrun".into();
        settings.window.exit_on_close_request = false;
        settings.window.max_size = Some([480, 240].into());
        settings.window.min_size = Some([480, 240].into());
        settings.window.position = Position::Centered;
        ProgressInfo::run(settings)?;
        let _ = std::fs::remove_dir(&temp_repo);
        Ok(())
    } else {
        log::info!("temp_repo: {:?}, deps_repo: {:?}", temp_repo, deps_repo);
        run_bundle_inner(&temp_repo, &deps_repo, &bundle_path, &mut None).await?;
        log::info!("Cleaning up temp repo: {:?}", temp_repo);
        let _ = std::fs::remove_dir(&temp_repo);
        Ok(())
    }
}

use flatpak_unsandbox::{CmdArg, FlatpakInfo, UnsandboxError};

pub async fn run_bundle_inner(
    temp_repo: &Path,
    deps_repo: &Path,
    bundle_path: &Path,
    sender: &mut Option<&mut Sender<gui::Message>>,
) -> Result<(), FlatrunError> {
    println!("Unsandboxing...");

    let info = FlatpakInfo::new()?;

    let command = vec![
        CmdArg::new_path("/app/libexec/flatrun-agent"),
        CmdArg::new_string("install-run-bundle".into()),
        CmdArg::new_path(temp_repo),
        CmdArg::new_path(deps_repo),
        CmdArg::new_path(bundle_path),
    ];
    let envs = vec![("RUST_LOG".to_string(), CmdArg::new_string("DEBUG".into()))];

    let mut cmd = info.run_unsandboxed(command, Some(envs), None)?;
    let mut child = cmd.stdout(Stdio::piped()).spawn().unwrap();
    let stdout = child.stdout.take().unwrap();

    // Stream output.
    let lines = BufReader::new(stdout).lines();
    for line in lines {
        let l = match line {
            Ok(a) => a,
            Err(_) => break,
        };
        let update_metadata = l.split("::").map(|x| x.to_string()).collect::<Vec<_>>();
        println!("GOT LINE: {:?}", update_metadata);
        if update_metadata.len() != 5 {
            if l.contains("RUNNING_APPLICATION") {
                if let Some(s) = sender {
                    log::info!("Sending hide command!");
                    if let Err(e) = s
                        .send(gui::Message::Finished(Pid::from_child(&child)))
                        .await
                    {
                        log::error!("{:?}", e);
                        break;
                    }
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
            if let Err(e) = s
                .send(gui::Message::Progress((
                    repo,
                    action,
                    app_ref,
                    message,
                    progress as f32 / 100.0,
                )))
                .await
            {
                log::error!("{:?}", e);
                break;
            }
        }
    }

    if let Some(s) = sender {
        log::info!("Sending hide command!");
        if let Err(e) = s.send(gui::Message::Close).await {
            log::error!("{:?}", e);
        }
    }
    let _ = std::fs::remove_dir(&temp_repo);
    Ok(())
}
