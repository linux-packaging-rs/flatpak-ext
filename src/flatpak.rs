use std::{
    env,
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};

use libflatpak::{
    glib::object::ObjectExt,
    prelude::{
        InstallationExt, InstallationExtManual, InstalledRefExt, InstanceExt, RefExt, RemoteExt,
        TransactionExt, TransactionExtManual,
    },
    Installation, LaunchFlags, Remote,
};
use rustix::process::{waitpid, Pid, WaitOptions};

use tempfile::{tempdir_in, TempDir};

use crate::{flathub::flathub_remote, PortapakError};

#[derive(Debug)]
pub struct FlatpakRepo {
    pub repo: TempDir,
    pub installation: libflatpak::Installation,
}

#[derive(Debug)]
pub struct Flatpak {
    app_ref: String,
    app_id: String,
}

impl FlatpakRepo {
    pub fn new() -> Result<Self, PortapakError> {
        let base_path = env::var("XDG_CACHE_HOME")
            .map_or(Path::new(&env::var("HOME").unwrap()).join(".cache"), |x| {
                Path::new(&x).to_path_buf()
            });
        let repo = tempdir_in(base_path)?;
        let repo_file = libflatpak::gio::File::for_path(repo.path());
        // Create installation
        let installation = libflatpak::Installation::for_path(
            &repo_file,
            true,
            libflatpak::gio::Cancellable::current().as_ref(),
        )?;
        // Add flathub
        installation.add_remote(
            &flathub_remote()?,
            false,
            libflatpak::gio::Cancellable::current().as_ref(),
        )?;
        log::debug!("Created a flatpak repository at {:?}", repo.path());
        Ok(Self { repo, installation })
    }
}

impl Flatpak {
    pub fn new(app_path: PathBuf, repo: &FlatpakRepo) -> Result<Self, PortapakError> {
        if !app_path.exists() {
            return Err(PortapakError::FileNotFound(app_path));
        }
        let app_path_file = libflatpak::gio::File::for_path(&app_path);
        log::info!("Installing {:?}...", &app_path);
        let transaction = libflatpak::Transaction::for_installation(
            &repo.installation,
            libflatpak::gio::Cancellable::current().as_ref(),
        )?;
        // Set up transaction
        transaction.add_default_dependency_sources();
        transaction.add_install_bundle(&app_path_file, None)?;
        // Set up connections to signals
        transaction.connect_operation_error(|a, b, c, d| {
            log::debug!(
                "Operation error: {:?} {:?} {:?} {:?}. Stopping transation...",
                a,
                b,
                c,
                d
            );
            false
        });
        transaction.connect_operation_done(|_, b, c, _| {
            log::debug!(
                "OPERATION: {:?} TYPE: {:?} SHORT_COMMIT: {}",
                b,
                b.operation_type(),
                &c[..6]
            );
            log::trace!("METADATA:");
            for l in b.metadata().unwrap().to_data().lines() {
                log::trace!("{}", l);
            }
        });
        // Run transaction
        transaction.run(libflatpak::gio::Cancellable::current().as_ref())?;
        let op = transaction.operations().last().unwrap().clone();
        let app_ref = op.get_ref().unwrap();
        let app_id = op.metadata().unwrap().string("Application", "name")?;
        log::debug!("Successfully installed ref {}", app_ref);
        log::debug!("Installed Applications:");
        let _ = repo
            .installation
            .list_installed_refs(libflatpak::gio::Cancellable::current().as_ref())?
            .iter()
            .map(|e| {
                log::debug!(
                    "Name: {:?} | Branch: {:?} | Version: {:?} | Size (MiB): {:.3} | Deployed: {:?}",
                    e.name().unwrap_or_default(),
                    e.branch().unwrap_or_default(),
                    e.appdata_version().unwrap_or_default(),
                    e.installed_size() as f64 / 1048576.0,
                    e.deploy_dir().unwrap_or_default(),
                )
            })
            .collect::<Vec<_>>();
        Ok(Self {
            app_ref: app_ref.into(),
            app_id: app_id.into(),
        })
    }

    pub fn run(&self, repo: &FlatpakRepo) -> Result<(), PortapakError> {
        let inst = repo.installation.launch_full(
            LaunchFlags::NONE,
            &self.app_id,
            None,
            Some(&self.branch()),
            None,
            libflatpak::gio::Cancellable::current().as_ref(),
        );
        match inst {
            Ok(i) => {
                while i.is_running() {
                    sleep(Duration::from_millis(1000));
                }
                log::info!("Instance is no longer running! Removing repo...");
                Ok(())
            }
            Err(e) => {
                log::error!("{}", e);
                Ok(())
            }
        }
    }

    fn branch(&self) -> String {
        self.app_ref.rsplit_once("/").unwrap().1.to_string()
    }
}
