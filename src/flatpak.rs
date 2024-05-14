use std::{
    env,
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};

use libflatpak::{
    prelude::InstanceExt,
    prelude::{InstallationExtManual, TransactionExt},
    LaunchFlags,
};
use rustix::process::{waitpid, Pid, WaitOptions};

use tempfile::{tempdir_in, TempDir};

use crate::PortapakError;

#[derive(Debug)]
pub struct FlatpakRepo {
    pub repo: TempDir,
    pub installation: libflatpak::Installation,
}

#[derive(Debug)]
pub struct Flatpak {
    app_ref: String,
    app_id: String,
    app_commit: String,
}

impl FlatpakRepo {
    pub fn new() -> Result<Self, PortapakError> {
        let base_path = env::var("XDG_CACHE_HOME")
            .map_or(Path::new(&env::var("HOME").unwrap()).join(".cache"), |x| {
                Path::new(&x).to_path_buf()
            });
        let repo = tempdir_in(base_path)?;
        log::debug!("random repo: {:?}", repo.path());
        let repo_file = libflatpak::gio::File::for_path(repo.path());
        let installation = libflatpak::Installation::for_path(
            &repo_file,
            true,
            libflatpak::gio::Cancellable::current().as_ref(),
        )?;
        log::debug!("got flatpak installation! {:?}", installation);
        Ok(Self { repo, installation })
    }
}

impl Flatpak {
    pub fn new(app_path: PathBuf, repo: &FlatpakRepo) -> Result<Self, PortapakError> {
        if !app_path.exists() {
            return Err(PortapakError::FileNotFound(app_path));
        }
        log::debug!("app path {:?} exists!", app_path);
        let app_path_file = libflatpak::gio::File::for_path(app_path);
        let transaction = libflatpak::Transaction::for_installation(
            &repo.installation,
            libflatpak::gio::Cancellable::current().as_ref(),
        )?;
        let _ = transaction.add_install_bundle(&app_path_file, None)?;
        let _ = transaction.add_default_dependency_sources();
        let _ = transaction.run(libflatpak::gio::Cancellable::current().as_ref())?;
        let op = transaction.operations().get(0).unwrap().clone();
        let app_ref = op.get_ref().unwrap();
        let app_commit = op.commit().unwrap();
        let app_id = op.metadata().unwrap().string("Application", "name")?;
        log::debug!("Installed bundle!");
        Ok(Self {
            app_ref: app_ref.into(),
            app_id: app_id.into(),
            app_commit: app_commit.into(),
        })
    }

    pub fn run(&self, repo: &FlatpakRepo) -> Result<(), PortapakError> {
        log::debug!("{:?}", self);
        let inst = repo.installation.launch_full(
            LaunchFlags::NONE,
            &self.app_id,
            None,
            Some(&self.branch()),
            Some(&self.app_commit),
            libflatpak::gio::Cancellable::current().as_ref(),
        )?;
        while inst.is_running() {
            sleep(Duration::from_millis(1000));
        }
        log::info!("Instance is no longer running! Removing repo...");
        Ok(())
    }

    fn branch(&self) -> String {
        self.app_ref.rsplit_once("/").unwrap().1.to_string()
    }
}
