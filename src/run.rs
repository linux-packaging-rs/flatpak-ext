use std::{env, path::PathBuf, process::Command};

use crate::{
    config::{FlatpakHandle, RunConfig, UserConfig},
    PortapakError,
};

#[derive(Debug, Clone)]
pub struct Flatpak {
    pub path: PathBuf,
    pub appid: String,
    pub config: RunConfig,
}

impl Flatpak {
    pub fn new(app_path: PathBuf, config: UserConfig) -> Result<Self, PortapakError> {
        if !app_path.exists() {
            return Err(PortapakError::FileNotFound(app_path));
        }

        let appid = app_path.file_stem().unwrap().to_string_lossy().to_string();
        Ok(Self {
            path: app_path.clone(),
            appid,
            config: config.get_config(FlatpakHandle::Path(app_path.clone())),
        })
    }

    pub fn extract_repo(
        &self,
        tmp_dir: PathBuf,
        out_dir: PathBuf,
    ) -> Result<PathBuf, PortapakError> {
        let temporary_repo = tmp_dir.join(&format!("portapak/repo/{}", self.appid));
        let out_repo = out_dir.join(&format!("portapak/repo/{}", self.appid));
        let repo_string = temporary_repo.as_os_str().to_string_lossy();
        let outdir_string = out_repo.as_os_str().to_string_lossy();
        if Command::new("ostree")
            .arg("init")
            .arg(&format!("--repo={}", repo_string))
            .arg("--mode=bare-user")
            .status()?.success() && Command::new("ostree")
            .arg("static-delta")
            .arg("apply-offline")
            .arg(&format!("--repo={}", repo_string))
            .arg(self.path.as_os_str())
            .status()?.success() && Command::new("sh")
            .arg("-c")
            .arg(&format!("ostree checkout --repo={} -U $(basename $(echo repo/objects/*/*.commit | cut -d/ -f3- --output-delimiter= ) .commit) {}", repo_string, outdir_string))
            .status()?.success() {
        Ok(out_repo)
        }
        else {
            Err(PortapakError::CommandUnsuccessful)
        }
    }

    pub fn run_self(&self, from_repo: Option<PathBuf>) -> Result<(), PortapakError> {
        if let Some(repo) = from_repo {
            if Command::new("flatpak")
                .env("HOME", self.config.get_home_directory().as_os_str())
                .arg(&format!("--env=HOME={}", env::var("HOME").unwrap()))
                .arg(&format!(
                    "--app-path={}",
                    repo.as_os_str().to_string_lossy()
                ))
                .arg("run")
                .arg(&self.appid)
                .status()?
                .success()
            {
                Ok(())
            } else {
                Err(PortapakError::CommandUnsuccessful)
            }
        } else {
            if Command::new("flatpak")
                .arg("run")
                .arg(&self.appid)
                .status()?
                .success()
            {
                Ok(())
            } else {
                Err(PortapakError::CommandUnsuccessful)
            }
        }
    }
}

pub fn run_flatpak(flatpak: Flatpak, config: UserConfig) -> Result<(), PortapakError> {
    let tmp = config.get_temporary_dir();
    let (tmp_dir, out_dir) = (tmp.join("tmp"), tmp.join("out"));
    let repo = flatpak.extract_repo(tmp_dir, out_dir)?;
    flatpak.run_self(Some(repo))?;
    Ok(())
}
