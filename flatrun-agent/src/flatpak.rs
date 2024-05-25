use std::{
    io,
    path::PathBuf,
    thread::{self},
};

use libflatpak::{
    gio::prelude::FileExt,
    glib::{KeyFile, KeyFileFlags},
    prelude::{
        BundleRefExt, InstallationExt, InstallationExtManual, InstanceExt, RefExt, TransactionExt,
    },
    BundleRef, Installation, LaunchFlags,
};
use rustix::{process::WaitOptions, thread::Pid};
use signal_hook::{consts::SIGINT, iterator::Signals};

use crate::FlatrunAgentError;

pub fn install_bundle(
    installation: PathBuf,
    _deps_installation: PathBuf,
    path: PathBuf,
) -> Result<(), FlatrunAgentError> {
    let bundle_install = get_repo(installation)?;
    let bundle_transaction = libflatpak::Transaction::for_installation(
        &bundle_install,
        libflatpak::gio::Cancellable::current().as_ref(),
    )?;
    // FIXME: https://github.com/ryanabx/flatrun/issues/6 (Upstream libflatpak issue)
    // let dep_install = get_repo(deps_installation, true)?;
    // TODO: Remove the below two lines and uncomment above line when issue is resolved.
    bundle_transaction.add_default_dependency_sources();
    let dep_install = system_repo()?;
    let dep_transaction = libflatpak::Transaction::for_installation(
        &dep_install,
        libflatpak::gio::Cancellable::current().as_ref(),
    )?;
    // Set up operations
    if !path.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, path.to_string_lossy()).into());
    }
    let app_path_file = libflatpak::gio::File::for_path(&path);
    let bundle = BundleRef::new(&app_path_file)?;
    let metadata = KeyFile::new();
    metadata.load_from_bytes(&bundle.metadata().unwrap(), KeyFileFlags::empty())?;
    let app_name = bundle.name().unwrap().to_string();
    let branch = bundle.branch().unwrap();
    let runtime_name = metadata
        .string("Application", "runtime")
        .unwrap()
        .to_string();
    if let Err(e) =
        dep_transaction.add_install("flathub", &format!("runtime/{}", runtime_name), &[])
    {
        log::warn!("{e}");
    }
    log::debug!("{}", dep_install.path().unwrap().uri());
    // FIXME: See above fixme
    // bundle_transaction.add_sideload_repo(
    //     &dep_install
    //         .path()
    //         .unwrap()
    //         .path()
    //         .unwrap()
    //         .to_string_lossy(),
    // );
    // bundle_transaction.add_install("flathub", &format!("runtime/{}", runtime_name), &[])?;
    log::debug!("Runtime: {}", &format!("runtime/{}", runtime_name));
    bundle_transaction.add_install_bundle(&app_path_file, None)?;
    // Connect operations to print
    dep_transaction.connect_new_operation(move |_, transaction, progress| {
        let current_action = format!(
            "{}::{}",
            transaction.operation_type().to_str().unwrap(),
            transaction.get_ref().unwrap()
        );
        println!(
            "DEPS::{}::{}::{}",
            current_action,
            progress.status().unwrap_or_default().to_string(),
            progress.progress()
        );
        progress.connect_changed(move |progress| {
            println!(
                "DEPS::{}::{}::{}",
                current_action,
                progress.status().unwrap_or_default().to_string(),
                progress.progress()
            );
        });
    });
    bundle_transaction.connect_new_operation(move |_, transaction, progress| {
        let current_action = format!(
            "{}::{}",
            transaction
                .operation_type()
                .to_str()
                .unwrap()
                .to_uppercase(),
            transaction.get_ref().unwrap()
        );
        println!(
            "TEMP::{}::{}::{}",
            current_action,
            progress.status().unwrap_or_default().to_string(),
            progress.progress()
        );
        progress.connect_changed(move |progress| {
            println!(
                "TEMP::{}::{}::{}",
                current_action,
                progress.status().unwrap_or_default().to_string(),
                progress.progress()
            );
        });
    });
    // Run operations
    dep_transaction.run(libflatpak::gio::Cancellable::current().as_ref())?;
    log::debug!("Installing application {:?}", app_name);
    bundle_transaction.run(libflatpak::gio::Cancellable::current().as_ref())?;
    log::debug!("{}, {}", app_name, runtime_name);
    log::debug!(
        "temp_installation: {:?}",
        bundle_install
            .list_installed_refs(libflatpak::gio::Cancellable::current().as_ref())?
            .iter()
            .map(|x| { x.format_ref().unwrap() })
            .collect::<Vec<_>>()
    );
    // Signal to gui to hide
    println!("RUNNING_APPLICATION");
    // Run bundle
    let inst = bundle_install
        .launch_full(
            LaunchFlags::DO_NOT_REAP,
            &app_name,
            None,
            Some(&branch),
            None,
            libflatpak::gio::Cancellable::current().as_ref(),
        )
        .unwrap();

    let pid = Pid::from_raw(inst.pid()).unwrap();
    let mut signals = Signals::new(&[SIGINT])?;

    thread::spawn(move || {
        for sig in signals.forever() {
            log::info!("Received signal {:?}", sig);
            let _ =
                rustix::process::kill_process(pid, rustix::process::Signal::from_raw(sig).unwrap());
        }
    });

    while !rustix::process::waitpid(Some(pid), WaitOptions::empty())
        .is_ok_and(|x| x.is_some_and(|y| y.exited() || y.signaled()))
    {}
    Ok(())
}

fn system_repo() -> Result<Installation, FlatrunAgentError> {
    Ok(libflatpak::Installation::new_system(
        libflatpak::gio::Cancellable::current().as_ref(),
    )?)
}

fn get_repo(repo: PathBuf) -> Result<Installation, FlatrunAgentError> {
    let repo_file = libflatpak::gio::File::for_path(repo);
    // Create installation
    let installation = libflatpak::Installation::for_path(
        &repo_file,
        true,
        libflatpak::gio::Cancellable::current().as_ref(),
    )?;
    Ok(installation)
}
