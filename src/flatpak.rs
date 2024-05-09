use std::{
    env,
    fs::{read_dir, remove_dir_all},
    path::{Path, PathBuf},
    process::Command,
};

use rand::{distributions::Alphanumeric, thread_rng, Rng};

use crate::PortapakError;

#[derive(Debug, Clone)]
pub struct Flatpak {
    pub path: PathBuf,
    pub appid: String,
    pub repo: PathBuf,
}

impl Flatpak {
    pub fn new(app_path: PathBuf) -> Result<Self, PortapakError> {
        if !app_path.exists() {
            return Err(PortapakError::FileNotFound(app_path));
        }
        log::debug!("app path {:?} exists!", app_path);
        let random_repo: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(20)
            .map(char::from)
            .collect();
        let repo = Path::new(&format!(
            "{}/portapak_{}",
            env::var("TMPDIR").unwrap_or(env::var("TEMPDIR").unwrap_or(
                env::var("TMP").unwrap_or(env::var("TEMP").unwrap_or("/tmp/".to_string()))
            )),
            random_repo,
        ))
        .to_path_buf();
        log::debug!("random repo: {:?}", repo);
        let status = Command::new("flatpak")
            .arg("install")
            .arg("--user")
            .arg(&app_path)
            .arg("-y")
            .env("FLATPAK_USER_DIR", &repo)
            .output()?;
        log::debug!(
            "command flatpak install {:?} -y ended with code {:?}",
            &app_path,
            status
        );
        if !repo.exists() {
            return Err(PortapakError::FileNotFound(repo));
        }
        let appid = read_dir(repo.join("app"))?
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
            .env("FLATPAK_USER_DIR", &self.repo)
            .status();
        let _ = remove_dir_all(&self.repo);
        match status {
            Ok(s) => {
                log::debug!("Flatpak exited with status {:?}", s);
                Ok(())
            }
            Err(e) => Err(PortapakError::from(e)),
        }
    }
}
