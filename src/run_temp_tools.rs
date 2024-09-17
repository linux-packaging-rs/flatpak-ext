use libflatpak::{
    glib::{KeyFile, KeyFileFlags},
    prelude::RemoteExt,
    prelude::{
        BundleRefExt, FileExt, InstallationExt, InstallationExtManual, InstanceExt, RefExt,
        RemoteRefExt, TransactionExt,
    },
    LaunchFlags, TransactionOperationType,
};
use rustix::process::{Pid, WaitOptions};
use signal_hook::{consts::SIGINT, iterator::Signals};
use std::thread;

use crate::types::{get_installation, Flatpak, FlatpakExtError, FlatpakOut, Remote, Repo};

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
) -> Result<(), FlatpakExtError> {
    log::debug!("Get the flatpak installations, error out if they don't exist or some other error occurs...");
    let deps_repo: libflatpak::Installation =
        get_installation(&deps_at.as_ref().unwrap_or(&Repo::default()))?;
    let install_repo: libflatpak::Installation = get_installation(&install_at)?;
    log::debug!("Add remote for installations");
    let remote = remote_uri.map_or(Remote::default(), |x| Remote::new(x));
    let default_branch = remote.clone().default_branch;
    let remote = libflatpak::Remote::try_from(remote)?;
    remote.set_default_branch(&default_branch);
    if let Err(e) = deps_repo.add_remote(
        &remote,
        true,
        libflatpak::gio::Cancellable::current().as_ref(),
    ) {
        log::error!("There was a problem adding the remote: {}", e);
    }
    if let Err(e) = install_repo.add_remote(
        &remote,
        true,
        libflatpak::gio::Cancellable::current().as_ref(),
    ) {
        log::error!("There was a problem adding the remote: {}", e);
    }
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
                    Ok::<FlatpakOut, FlatpakExtError>(
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
                    Ok::<FlatpakOut, FlatpakExtError>(
                        Flatpak::Download(app_id)
                            .convert_to_flatpak_out(&deps_repo, &remote, &branch, true)?,
                    )
                }
            }
        },
        |x| {
            Ok::<FlatpakOut, FlatpakExtError>(x.convert_to_flatpak_out(
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
            .map_err(|e| FlatpakExtError::from(e)),
        FlatpakOut::Download(ref download) => deps_transaction
            .add_install(
                &download.remote_name().unwrap(),
                &download.format_ref().unwrap(),
                &[],
            )
            .map_err(|e| FlatpakExtError::from(e)),
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
            .map_err(|e| FlatpakExtError::from(e)),
        FlatpakOut::Download(ref download) => install_transaction
            .add_install(
                &download.remote_name().unwrap(),
                &download.format_ref().unwrap(),
                &[],
            )
            .map_err(|e| FlatpakExtError::from(e)),
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
