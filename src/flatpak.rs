use std::{fs::read_dir, path::PathBuf, process::Command};

use tempfile::{tempdir, TempDir};

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
        let repo = tempdir()?;
        log::debug!("random repo: {:?}", repo);
        let status = Command::new("flatpak")
            .arg("install")
            .arg("--user")
            .arg(&app_path)
            .arg("-y")
            .env("FLATPAK_USER_DIR", repo.path())
            .output()?;
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
