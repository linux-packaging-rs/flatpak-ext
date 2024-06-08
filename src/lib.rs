//!
//! libflatrun provides a library to run flatpaks without installing them.
//! https://github.com/ryanabx/flatrun
//!

use libflatpak::{
    glib::{KeyFile, KeyFileFlags},
    prelude::RemoteExt,
    prelude::{
        BundleRefExt, FileExt, InstallationExt, InstallationExtManual, InstanceExt, RefExt,
        RemoteRefExt, TransactionExt,
    },
    BundleRef, LaunchFlags, RefKind, TransactionOperationType,
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use rustix::process::{Pid, WaitOptions};
use signal_hook::{consts::SIGINT, iterator::Signals};
use std::{env, fs::remove_dir_all, path::PathBuf, thread};

#[derive(Debug)]
pub enum FlatrunError {
    Glib(libflatpak::glib::Error),
    IO(std::io::Error),
    Reqwest(reqwest::Error),
}

impl From<std::io::Error> for FlatrunError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}

impl From<libflatpak::glib::Error> for FlatrunError {
    fn from(value: libflatpak::glib::Error) -> Self {
        Self::Glib(value)
    }
}

impl From<reqwest::Error> for FlatrunError {
    fn from(value: reqwest::Error) -> Self {
        Self::Reqwest(value)
    }
}

#[derive(Clone, Debug)]
pub enum Flatpak {
    Bundle(PathBuf),
    Download(String),
}

#[derive(Clone, Debug)]
enum FlatpakOut {
    Bundle(libflatpak::BundleRef),
    Download(libflatpak::RemoteRef),
}

impl Flatpak {
    fn convert_to_flatpak_out(
        &self,
        installation: &libflatpak::Installation,
        remote: &libflatpak::Remote,
        branch: &String,
        is_runtime: bool,
    ) -> Result<FlatpakOut, FlatrunError> {
        match self {
            Flatpak::Bundle(path) => {
                let bundle_path = libflatpak::gio::File::for_path(&path);
                let bundle = BundleRef::new(&bundle_path)?;
                Ok(FlatpakOut::Bundle(bundle))
            }
            Flatpak::Download(app_id) => {
                Ok(FlatpakOut::Download(installation.fetch_remote_ref_sync(
                    &remote.name().unwrap(),
                    if is_runtime {
                        RefKind::Runtime
                    } else {
                        RefKind::App
                    },
                    &app_id,
                    libflatpak::default_arch().as_deref(),
                    Some(&branch),
                    libflatpak::gio::Cancellable::current().as_ref(),
                )?))
            }
        }
    }
}

#[derive(Clone, Debug)]
/// A remote to download from
struct Remote {
    /// uri to a .flatpakrepo file (can be a URL or a file path)
    uri: String,
    name: String,
    default_branch: String,
}

impl Default for Remote {
    fn default() -> Self {
        Remote {
            uri: "https://dl.flathub.org/repo/flathub.flatpakrepo".into(),
            name: "flathub".into(),
            default_branch: "stable".into(),
        }
    }
}

impl Remote {
    fn new(uri: String) -> Self {
        Remote {
            uri: uri.clone(),
            name: uri.clone(),
            default_branch: "master".into(),
        }
    }
}

impl TryFrom<Remote> for libflatpak::Remote {
    fn try_from(value: Remote) -> Result<Self, Self::Error> {
        log::trace!("Loading bytes from uri: '{}'", value.uri);
        let bytes = uri_to_bytes(value.uri)?;
        let remote = libflatpak::Remote::from_file(&value.name, &bytes)?;
        if remote.name().unwrap().to_string() == "flathub".to_string() {
            remote.set_default_branch("stable");
        }
        Ok(remote)
    }

    type Error = FlatrunError;
}

pub fn uri_to_bytes(uri: String) -> Result<libflatpak::glib::Bytes, FlatrunError> {
    if uri.starts_with("file://") {
        Ok(
            libflatpak::gio::File::for_path(&uri.split_once("file://").unwrap().0)
                .load_bytes(libflatpak::gio::Cancellable::current().as_ref())?
                .0,
        )
    } else {
        Ok(libflatpak::glib::Bytes::from_owned(
            reqwest::blocking::get(uri)?.bytes().unwrap(),
        ))
    }
}

#[derive(Clone, Debug, Default)]
/// An abstraction representing a flatpak repository
pub enum Repo {
    /// A temporary repo that is intended to be deleted when the value is dropped
    /// > **WARNING:** Only enter in a directory that should be deleted when dropped
    Temp(PathBuf),
    /// The default System repo
    #[default]
    System,
    /// The default User repo
    User,
    /// A static repo that will persist after the execution of this program
    /// `user`=`true` for `--user`, `user`=`false` for `--system`
    Static { path: PathBuf, user: bool },
}

impl Repo {
    /// Creates a new temp repo in the default tmp directory
    pub fn temp() -> Self {
        let path = env::temp_dir();
        Self::temp_in(path)
    }

    /// Creates a new temp repo in the specified directory
    pub fn temp_in(path: PathBuf) -> Self {
        let foldername: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();
        let temp_repo = path.join(format!(".tmp{}", foldername));
        Repo::Temp(temp_repo)
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        if let Self::Temp(path) = self {
            log::debug!("Dropping TempRepo: {}", &path.to_string_lossy());
            let _ = remove_dir_all(&path);
        }
    }
}

pub fn get_installation(value: &Repo) -> Result<libflatpak::Installation, FlatrunError> {
    match value {
        Repo::Temp(ref path) => {
            let repo_file = libflatpak::gio::File::for_path(path);
            // Create installation
            Ok(libflatpak::Installation::for_path(
                &repo_file,
                true,
                libflatpak::gio::Cancellable::current().as_ref(),
            )?)
        }
        Repo::Static { ref path, user } => {
            let repo_file = libflatpak::gio::File::for_path(path);
            Ok(libflatpak::Installation::for_path(
                &repo_file,
                *user,
                libflatpak::gio::Cancellable::current().as_ref(),
            )?)
        }
        Repo::System => Ok(libflatpak::Installation::new_system(
            libflatpak::gio::Cancellable::current().as_ref(),
        )?),
        Repo::User => Ok(libflatpak::Installation::new_user(
            libflatpak::gio::Cancellable::current().as_ref(),
        )?),
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    Install {
        r: String,
        progress: f32,
        dependency: bool,
    },
    Running {
        n: String,
    },
    Unknown,
}

impl Message {
    fn new_from(t: TransactionOperationType, r: String, p: f32, d: bool) -> Self {
        match t {
            TransactionOperationType::Install | TransactionOperationType::InstallBundle => {
                Self::Install {
                    r,
                    progress: p,
                    dependency: d,
                }
            }
            _ => Self::Unknown,
        }
    }
}

/// Runs a flatpak
pub fn run(
    install_at: Repo,
    app: Flatpak,
    deps_at: Option<Repo>,
    runtime: Option<Flatpak>,
    remote_uri: Option<String>,
    update_callback: fn(Message),
) -> Result<(), FlatrunError> {
    log::debug!("Get the flatpak installations, error out if they don't exist or some other error occurs...");
    let deps_repo: libflatpak::Installation =
        get_installation(&deps_at.as_ref().unwrap_or(&Repo::default()))?;
    let install_repo: libflatpak::Installation = get_installation(&install_at)?;
    log::debug!("Add remote for installations");
    let remote = remote_uri.map_or(Remote::default(), |x| Remote::new(x));
    let default_branch = remote.clone().default_branch;
    let remote = libflatpak::Remote::try_from(remote)?;
    remote.set_gpg_verify(false);
    remote.set_default_branch(&default_branch);
    deps_repo.add_remote(
        &remote,
        true,
        libflatpak::gio::Cancellable::current().as_ref(),
    )?;
    install_repo.add_remote(
        &remote,
        true,
        libflatpak::gio::Cancellable::current().as_ref(),
    )?;
    log::debug!("Get the flatpak");
    let app = app.convert_to_flatpak_out(&install_repo, &remote, &default_branch, false)?;
    log::debug!("Get the runtime");
    let runtime = runtime.map_or(
        {
            match app {
                FlatpakOut::Bundle(ref bundle) => {
                    let config = KeyFile::new();
                    config.load_from_bytes(&bundle.metadata().unwrap(), KeyFileFlags::NONE)?;
                    let runtime_str = config.string("Application", "runtime").unwrap().to_string();
                    let mut info = runtime_str.split("/");
                    let app_id = info.next().unwrap().to_string();
                    let _ = info.next().unwrap().to_string();
                    let branch = info.next().unwrap().to_string();
                    Ok::<FlatpakOut, FlatrunError>(
                        Flatpak::Download(app_id)
                            .convert_to_flatpak_out(&deps_repo, &remote, &branch, true)?,
                    )
                }
                FlatpakOut::Download(ref download) => {
                    let config = KeyFile::new();
                    config.load_from_bytes(&download.metadata().unwrap(), KeyFileFlags::NONE)?;
                    let runtime_str = config.string("Application", "runtime").unwrap().to_string();
                    let mut info = runtime_str.split("/");
                    let app_id = info.next().unwrap().to_string();
                    let _ = info.next().unwrap().to_string();
                    let branch = info.next().unwrap().to_string();
                    Ok::<FlatpakOut, FlatrunError>(
                        Flatpak::Download(app_id)
                            .convert_to_flatpak_out(&deps_repo, &remote, &branch, true)?,
                    )
                }
            }
        },
        |x| {
            Ok::<FlatpakOut, FlatrunError>(x.convert_to_flatpak_out(
                &deps_repo,
                &remote,
                &default_branch,
                true,
            )?)
        },
    )?;
    log::debug!("Create transactions");
    let deps_transaction = libflatpak::Transaction::for_installation(
        &deps_repo,
        libflatpak::gio::Cancellable::current().as_ref(),
    )?;
    let install_transaction = libflatpak::Transaction::for_installation(
        &install_repo,
        libflatpak::gio::Cancellable::current().as_ref(),
    )?;
    log::debug!("Connect operations to callback");
    deps_transaction.connect_new_operation(move |_, transaction, progress| {
        let op_type = transaction.operation_type().clone();
        let app_ref = transaction.get_ref().unwrap().to_string();
        update_callback(Message::new_from(op_type, app_ref.clone(), 0.0, true));
        progress.connect_changed(move |progress| {
            update_callback(Message::new_from(
                op_type,
                app_ref.clone(),
                progress.progress() as f32 / 100.0,
                true,
            ));
        });
    });
    install_transaction.connect_new_operation(move |_, transaction, progress| {
        let op_type = transaction.operation_type().clone();
        let app_ref = transaction.get_ref().unwrap().to_string();
        update_callback(Message::new_from(op_type, app_ref.clone(), 0.0, false));
        progress.connect_changed(move |progress| {
            update_callback(Message::new_from(
                op_type,
                app_ref.clone(),
                progress.progress() as f32 / 100.0,
                false,
            ));
        });
    });
    log::debug!("Add installation command to dependency transaction");
    if let Err(e) = match runtime {
        FlatpakOut::Bundle(ref bundle) => deps_transaction
            .add_install_bundle(&bundle.file().unwrap(), None)
            .map_err(|e| FlatrunError::from(e)),
        FlatpakOut::Download(ref download) => deps_transaction
            .add_install(
                &download.remote_name().unwrap(),
                &download.format_ref().unwrap(),
                &[],
            )
            .map_err(|e| FlatrunError::from(e)),
    } {
        log::warn!("Could not install dependency: {:?}", e);
    }
    log::debug!("Run dependency transaction.");
    deps_transaction.run(libflatpak::gio::Cancellable::current().as_ref())?;
    log::debug!("Set up sideload repo...");
    install_transaction
        .add_sideload_repo(&deps_repo.path().unwrap().path().unwrap().to_string_lossy());
    log::debug!("Add installation command to install transaction");
    if let Err(e) = match app {
        FlatpakOut::Bundle(ref bundle) => install_transaction
            .add_install_bundle(&bundle.file().unwrap(), None)
            .map_err(|e| FlatrunError::from(e)),
        FlatpakOut::Download(ref download) => install_transaction
            .add_install(
                &download.remote_name().unwrap(),
                &download.format_ref().unwrap(),
                &[],
            )
            .map_err(|e| FlatrunError::from(e)),
    } {
        log::error!("Could not install app: {:?}", e);
        panic!()
    }
    log::debug!("Add deps as dependency source");
    install_transaction.add_dependency_source(&deps_repo);
    log::debug!("Run install transaction");
    install_transaction.run(libflatpak::gio::Cancellable::current().as_ref())?;
    log::debug!("Run instance");

    let inst = match app {
        FlatpakOut::Bundle(bundle) => install_repo.launch_full(
            LaunchFlags::DO_NOT_REAP,
            &bundle.name().unwrap(),
            bundle.arch().as_deref(),
            bundle.branch().as_deref(),
            None,
            libflatpak::gio::Cancellable::current().as_ref(),
        )?,
        FlatpakOut::Download(download) => install_repo.launch_full(
            LaunchFlags::DO_NOT_REAP,
            &download.name().unwrap(),
            download.arch().as_deref(),
            download.branch().as_deref(),
            None,
            libflatpak::gio::Cancellable::current().as_ref(),
        )?,
    };
    // Track instance
    let pid = Pid::from_raw(inst.pid()).unwrap();
    let mut signals = Signals::new(&[SIGINT])?;

    thread::spawn(move || {
        for sig in signals.forever() {
            log::info!("Received signal {:?}", sig);
            let _ =
                rustix::process::kill_process(pid, rustix::process::Signal::from_raw(sig).unwrap());
        }
    });

    log::debug!("Waiting on instance to close...");
    while !rustix::process::waitpid(Some(pid), WaitOptions::empty())
        .is_ok_and(|x| x.is_some_and(|y| y.exited() || y.signaled()))
    {}
    log::debug!("Drop repos");
    drop(install_repo);
    drop(deps_at);
    Ok(())
}
