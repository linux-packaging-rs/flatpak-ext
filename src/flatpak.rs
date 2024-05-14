use std::{
    env,
    fs::read_dir,
    path::{Path, PathBuf},
    process::Command,
};

use rustix::process;

use tempfile::{tempdir_in, TempDir};

use crate::PortapakError;

#[derive(Debug)]
pub struct Flatpak {
    pub path: PathBuf,
    pub appid: String,
    pub repo: TempDir,
}

impl Flatpak {
    pub fn new(app_path: PathBuf) -> Result<Self, PortapakError> {
        if !app_path.exists() {
            return Err(PortapakError::FileNotFound(app_path));
        }
        log::debug!("app path {:?} exists!", app_path);
        let base_path = env::var("XDG_CACHE_HOME")
            .map_or(Path::new(&env::var("HOME").unwrap()).join(".cache"), |x| {
                Path::new(&x).to_path_buf()
            });
        let repo = tempdir_in(base_path)?;
        log::debug!("random repo: {:?}", repo.path());
        log::info!(
            "FLATPAK_USER_DIR={:?} flatpak install --user {:?} -y",
            repo.path(),
            &app_path
        );
        let status = Command::new("flatpak")
            .arg("install")
            .arg("--user")
            .arg(&app_path)
            .arg("-y")
            .env("FLATPAK_USER_DIR", repo.path())
            .status()?;
        log::debug!(
            "command flatpak install {:?} -y ended with code {:?}",
            &app_path,
            status
        );
        let appid = read_dir(repo.path().join("app"))?
            .next()
            .unwrap()?
            .file_name()
            .to_string_lossy()
            .to_string();
        log::debug!("appid found: {}", appid);
        Ok(Self {
            path: app_path.clone(),
            appid,
            repo,
        })
    }

    pub fn run(&self) -> Result<(), PortapakError> {
        log::info!(
            "FLATPAK_USER_DIR={:?} flatpak run --user {:?}",
            &self.repo.path(),
            &self.appid
        );
        let status = Command::new("flatpak")
            .arg("run")
            .arg("--user")
            .arg(&self.appid)
            .env("FLATPAK_USER_DIR", &self.repo.path())
            .status();
        match status {
            Ok(s) => {
                log::debug!("Flatpak exited with status {:?}", s);
                Ok(())
            }
            Err(e) => Err(PortapakError::from(e)),
        }
    }
}
