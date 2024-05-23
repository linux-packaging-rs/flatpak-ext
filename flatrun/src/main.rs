use std::{
    env,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::Stdio,
    string::FromUtf8Error,
};

use ashpd::desktop::file_chooser::{FileFilter, SelectedFiles};
use async_std::task::spawn;
use clap::{arg, Args, Parser, Subcommand};
use flatpak_unsandbox::{Program, ProgramArg, UnsandboxError};
use slint::Weak;
use tempfile::TempDir;

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

slint::slint!(
    import { VerticalBox, HorizontalBox, Button, LineEdit, ProgressIndicator } from "std-widgets.slint";
export component MainWindow inherits Window {
    title: "Flatrun";
    icon: @image-url("../flatrun.png");
    callback start_job;
    in property <float> progress: 0.0;
    in property <string> repo: "REPO";
    in property <string> action: "ACTION";
    in property <string> app_ref: "app/com.example.Example";
    in property <string> message: "Example";
    default-font-size: 10px;
    VerticalLayout {
        Text {
            text: root.repo;
        }
        Text {
            text: root.action;
        }
        Text {
            text: root.app_ref;
        }
        Text {
            text: root.message;
        }
        ProgressIndicator {
            progress: root.progress;
        }
    }
}

);

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
        return Err(FlatrunError::FileNotFound(pth));
    }
    Ok(pth)
}

fn get_repos() -> Result<(TempDir, PathBuf), FlatrunError> {
    let temp_repo = TempDir::new_in(env::var("XDG_STATE_HOME").unwrap())?;
    let deps_repo = Path::new(&env::var("XDG_DATA_HOME").unwrap_or(format!(
        "{}/.local/share/flatrun",
        env::var("HOME").unwrap()
    )))
    .join("deps");
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
        Err(FlatrunError::CommandUnsuccessful(
            "[BY_PATH] No file specified".to_string(),
        ))
    }
}

async fn run_bundle(bundle_path: PathBuf, gui: bool) -> Result<(), FlatrunError> {
    let (temp_repo, deps_repo) = get_repos()?;
    log::info!(
        "temp_repo: {:?}, deps_repo: {:?}",
        temp_repo.path(),
        deps_repo
    );
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
        let mut child = cmd.stdout(Stdio::piped()).spawn().unwrap();
        let stdout = child.stdout.take().unwrap();
        let window = if gui {
            Some(MainWindow::new().unwrap())
        } else {
            None
        };
        let window_ref = window.as_ref().map(|x| x.as_weak());
        let handle = spawn(async move {
            // Stream output.
            let lines = BufReader::new(stdout).lines();
            for line in lines {
                let update_metadata = line
                    .unwrap()
                    .split("::")
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>();
                println!("GOT LINE: {:?}", update_metadata);
                if update_metadata.len() != 5 {
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
                if let Some(w) = window_ref.as_ref() {
                    w.upgrade_in_event_loop(move |window| {
                        window.set_repo(repo.into());
                        window.set_action(action.into());
                        window.set_app_ref(app_ref.into());
                        window.set_message(message.into());
                        window.set_progress(progress as f32 / 100.0);
                    })
                    .unwrap();
                }
            }
            // FIXME: https://github.com/slint-ui/slint/issues/4225
            // if let Some(w) = window_ref.as_ref() {
            //     w.upgrade_in_event_loop(|window| {
            //         window.hide().unwrap();
            //     })
            //     .unwrap();
            // }
        });
        if let Some(w) = window.as_ref() {
            w.run().unwrap();
        }
        handle.await;
    }
    log::info!("Cleaning up temp repo: {:?}", temp_repo.path());
    Ok(())
}
