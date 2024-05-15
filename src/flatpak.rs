use std::{
    env, fs,
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};

use indicatif::{ProgressBar, ProgressStyle};
use libflatpak::{
    glib::{object::ObjectExt, KeyFile, KeyFileFlags},
    prelude::RemoteRefExt,
    prelude::{
        BundleRefExt, InstallationExt, InstallationExtManual, InstalledRefExt, InstanceExt, RefExt,
        RemoteExt, TransactionExt, TransactionExtManual,
    },
    BundleRef, Installation, LaunchFlags, Remote,
};
use rustix::process::{waitpid, Pid, WaitOptions};

use tempfile::{tempdir_in, TempDir};

use crate::{remotes::flathub_remote, PortapakError, RefType};

#[derive(Debug)]
pub struct FlatpakRepo {
    pub repo: TempDir,
    pub installation: libflatpak::Installation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DependencyInstall {
    System,
    User,
    Temp,
}

impl From<&str> for DependencyInstall {
    fn from(value: &str) -> Self {
        match value {
            "system" => Self::System,
            "user" => Self::User,
            "temp" => Self::Temp,
            _ => {
                log::warn!("Value {} not valid. Choosing Temp...", value);
                Self::Temp
            }
        }
    }
}

#[derive(Debug)]
pub struct Flatpak {
    app_ref: String,
    app_id: String,
}

impl FlatpakRepo {
    pub fn new(offline: bool) -> Result<Self, PortapakError> {
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
        if !offline {
            // Add flathub
            installation.add_remote(
                &flathub_remote()?,
                false,
                libflatpak::gio::Cancellable::current().as_ref(),
            )?;
        }
        Ok(Self { repo, installation })
    }
}

impl Flatpak {
    pub fn new(
        ref_type: RefType,
        repo: &FlatpakRepo,
        deps_to: DependencyInstall,
        offline: bool,
    ) -> Result<Self, PortapakError> {
        let transaction = libflatpak::Transaction::for_installation(
            &repo.installation,
            libflatpak::gio::Cancellable::current().as_ref(),
        )?;
        // Install deps first
        let dep_install = match deps_to {
            DependencyInstall::System => libflatpak::Transaction::for_installation(
                &Installation::new_system(libflatpak::gio::Cancellable::current().as_ref())?,
                libflatpak::gio::Cancellable::current().as_ref(),
            )?,
            DependencyInstall::User => libflatpak::Transaction::for_installation(
                &Installation::new_user(libflatpak::gio::Cancellable::current().as_ref())?,
                libflatpak::gio::Cancellable::current().as_ref(),
            )?,
            DependencyInstall::Temp => libflatpak::Transaction::for_installation(
                &repo.installation,
                libflatpak::gio::Cancellable::current().as_ref(),
            )?,
        };
        dep_install.add_default_dependency_sources();
        // Set up transaction
        transaction.add_default_dependency_sources();
        match ref_type {
            RefType::Path { path } => {
                if !path.exists() {
                    return Err(PortapakError::FileNotFound(path));
                }
                let app_path_file = libflatpak::gio::File::for_path(&path);
                let bundle = BundleRef::new(&app_path_file)?;
                let app = bundle.name();
                log::info!("Installing {:?}...", &app);
                log::debug!("Bundle origin: {:?}", &bundle.origin());
                let metadata = KeyFile::new();
                metadata.load_from_bytes(&bundle.metadata().unwrap(), KeyFileFlags::empty())?;
                log::info!(
                    "Bundle metadata keys for Application: {:?}. Runtime: {:?} Ref: {:?}",
                    metadata.keys("Application"),
                    metadata.string("Application", "runtime"),
                    bundle.format_ref()
                );
                dep_install.add_install(
                    "flathub",
                    &format!(
                        "runtime/{}",
                        metadata.string("Application", "runtime").unwrap()
                    ),
                    &[],
                )?;
                transaction.add_install_bundle(&app_path_file, None)?;
            }
            RefType::Name { remote, app } => {
                let rmt = repo
                    .installation
                    .remote_by_name(&remote, libflatpak::gio::Cancellable::current().as_ref())?;
                let branch = Some(rmt.default_branch().unwrap_or("stable".into()));
                let arch = libflatpak::default_arch();
                let remote_ref = repo.installation.fetch_remote_ref_sync(
                    &remote,
                    libflatpak::RefKind::App,
                    &app,
                    arch.as_deref(),
                    branch.as_deref(),
                    libflatpak::gio::Cancellable::current().as_ref(),
                )?;
                let metadata = KeyFile::new();
                metadata.load_from_bytes(&remote_ref.metadata().unwrap(), KeyFileFlags::empty())?;
                dep_install.add_install(
                    "flathub",
                    &format!(
                        "runtime/{}",
                        metadata.string("Application", "runtime").unwrap()
                    ),
                    &[],
                )?;
                transaction.add_install(&remote, &remote_ref.format_ref().unwrap(), &[])?;
            }
        }
        // Set up connections to signals
        dep_install.connect_new_operation(|_, b, c| {
            let prog_bar = ProgressBar::new(100);
            prog_bar.set_style(ProgressStyle::default_spinner());
            prog_bar.set_message(c.status().unwrap_or_default().to_string());
            log::trace!("{}", b.metadata().unwrap().to_data());
            c.connect_changed(move |a| {
                prog_bar.set_position(a.progress() as u64);
                prog_bar.set_message(a.status().unwrap_or_default().to_string());
            });
        });
        transaction.connect_new_operation(|_, b, c| {
            let prog_bar = ProgressBar::new(100);
            prog_bar.set_style(ProgressStyle::default_spinner());
            prog_bar.set_message(c.status().unwrap_or_default().to_string());
            log::trace!("{}", b.metadata().unwrap().to_data());
            c.connect_changed(move |a| {
                prog_bar.set_position(a.progress() as u64);
                prog_bar.set_message(a.status().unwrap_or_default().to_string());
            });
        });
        // Run transaction
        if !offline {
            log::debug!("Installing deps");
            dep_install.run(libflatpak::gio::Cancellable::current().as_ref())?;
        }
        log::debug!("Installing application");
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
