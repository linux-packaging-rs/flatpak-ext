use std::{env, path::Path, process::Command};

use indicatif::{ProgressBar, ProgressStyle};
use libflatpak::{
    gio::{prelude::FileExt, File},
    glib::{KeyFile, KeyFileFlags},
    prelude::{
        BundleRefExt, InstallationExt, InstalledRefExt, RefExt, RemoteExt, RemoteRefExt,
        TransactionExt,
    },
    BundleRef, Installation,
};

use tempfile::{tempdir_in, TempDir};

use crate::{remotes::flathub_remote, FlatrunError, RefType};

#[derive(Debug)]
pub struct FlatpakRepo {
    pub repo: TempDir,
    pub installation: libflatpak::Installation,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum DependencyInstall {
    System,
    #[default]
    User,
    Temp,
}

impl From<&str> for DependencyInstall {
    fn from(value: &str) -> Self {
        match value {
            "system" => Self::System,
            "user" => Self::User,
            "temp" => Self::Temp,
            _ => Self::default(),
        }
    }
}

#[derive(Debug)]
pub struct Flatpak {
    app_ref: String,
    app_id: String,
}

impl FlatpakRepo {
    pub fn new(offline: bool) -> Result<Self, FlatrunError> {
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
    ) -> Result<Self, FlatrunError> {
        let app_install = libflatpak::Transaction::for_installation(
            &repo.installation,
            libflatpak::gio::Cancellable::current().as_ref(),
        )?;
        // Install deps first
        let runtime_install = match deps_to {
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
        runtime_install.add_default_dependency_sources();
        // Set up transaction
        app_install.add_default_dependency_sources();
        let (app_name, runtime_name) = match ref_type {
            RefType::Path { path } => {
                if !path.exists() {
                    return Err(FlatrunError::FileNotFound(path));
                }
                let app_path_file = libflatpak::gio::File::for_path(&path);
                let bundle = BundleRef::new(&app_path_file)?;
                let metadata = KeyFile::new();
                metadata.load_from_bytes(&bundle.metadata().unwrap(), KeyFileFlags::empty())?;
                let app_name = bundle.name().unwrap().to_string();
                let runtime_name = metadata
                    .string("Application", "runtime")
                    .unwrap()
                    .to_string();
                if let Err(e) = runtime_install.add_install(
                    "flathub",
                    &format!("runtime/{}", runtime_name),
                    &[],
                ) {
                    log::warn!("{e}");
                }
                app_install.add_install_bundle(&app_path_file, None)?;
                (app_name, runtime_name)
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
                let app_name = remote_ref.name().unwrap().to_string();
                let runtime_name = metadata
                    .string("Application", "runtime")
                    .unwrap()
                    .to_string();
                if let Err(e) = runtime_install.add_install(
                    "flathub",
                    &format!("runtime/{}", runtime_name),
                    &[],
                ) {
                    log::warn!("{e}");
                }
                app_install.add_install(&remote, &remote_ref.format_ref().unwrap(), &[])?;
                (app_name, runtime_name)
            }
        };
        // Set up connections to signals
        runtime_install.connect_new_operation(|_, b, c| {
            let prog_bar = ProgressBar::new(100);
            prog_bar.set_style(ProgressStyle::default_spinner());
            prog_bar.set_message(c.status().unwrap_or_default().to_string());
            log::trace!("{}", b.metadata().unwrap().to_data());
            c.connect_changed(move |a| {
                prog_bar.set_position(a.progress() as u64);
                prog_bar.set_message(a.status().unwrap_or_default().to_string());
            });
        });
        app_install.connect_new_operation(|_, b, c| {
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
            println!(
                "Installing runtime {:?} to {:?} (if it doesn't exist)",
                runtime_name, deps_to
            );
            if let Err(e) = runtime_install.run(libflatpak::gio::Cancellable::current().as_ref()) {
                log::warn!("{e}");
            }
        }
        println!("Installing application {:?}", app_name);
        app_install.run(libflatpak::gio::Cancellable::current().as_ref())?;
        let op = app_install.operations().last().unwrap().clone();
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

    pub fn run(&self, repo: &FlatpakRepo) -> Result<(), FlatrunError> {
        let data = File::for_path("/.flatpak-info");
        let flatpak_info = KeyFile::new();
        flatpak_info.load_from_bytes(
            &data
                .load_bytes(libflatpak::gio::Cancellable::current().as_ref())?
                .0,
            KeyFileFlags::empty(),
        )?;
        let flatrun_host_path = Path::new(
            &flatpak_info
                .string("Instance", "app-path")
                .unwrap()
                .to_string(),
        )
        .join("libexec/flatrun-host");
        let _ = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "flatpak-spawn --host {:?} run {:?} {} {}",
                flatrun_host_path,
                repo.repo.path(),
                &self.app_id,
                &self.branch()
            ))
            .status();
        Ok(())
    }

    fn branch(&self) -> String {
        self.app_ref.rsplit_once("/").unwrap().1.to_string()
    }
}
