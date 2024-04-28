use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

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
        let temporary_repo = tmp_dir.join(&self.appid);
        let out_repo = out_dir.join(&self.appid);
        fs::create_dir_all(&temporary_repo)?;
        fs::create_dir_all(&out_dir.parent().unwrap())?;
        let repo_string = temporary_repo.as_os_str().to_string_lossy();
        let outdir_string = out_repo.as_os_str().to_string_lossy();
        log::debug!("{} :: {}", repo_string, outdir_string);

        let command_1 = format!("ostree init --repo={} --mode=bare-user", repo_string);
        log::debug!("{}", command_1);
        if !Command::new("sh")
            .arg("-c")
            .arg(&command_1)
            .status()?
            .success()
        {
            fs::remove_dir_all(&temporary_repo)?;
            fs::remove_dir_all(&out_repo)?;
            return Err(PortapakError::CommandUnsuccessful(command_1));
        }

        let command_2 = format!(
            "ostree static-delta apply-offline --repo={} {}",
            repo_string,
            self.path.as_os_str().to_string_lossy()
        );
        log::debug!("{}", command_2);
        if !Command::new("sh")
            .arg("-c")
            .arg(&command_2)
            .status()?
            .success()
        {
            fs::remove_dir_all(&temporary_repo)?;
            fs::remove_dir_all(&out_repo)?;
            return Err(PortapakError::CommandUnsuccessful(command_2));
        }
        let command_2b = format!("ls {}/objects/*/*.commit", repo_string);
        log::debug!("{}", command_2b);
        let commit_path = Path::new(&String::from_utf8(
            Command::new("sh")
                .arg("-c")
                .arg(&command_2b)
                .output()?
                .stdout,
        )?)
        .to_path_buf();
        let commit = format!(
            "{}{}",
            commit_path
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .trim()
                .to_string(),
            commit_path
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .trim()
                .to_string()
        );
        log::debug!("'{}'", commit);
        // FIXME: Hacky workaround, ostree should have a better way to check out branches when applying offline (https://github.com/flatpak/flatpak/issues/126#issuecomment-227068860)
        let command_3 = format!(
            "ostree checkout --repo={} -U {} {}",
            repo_string, commit, outdir_string
        );
        log::debug!("{}", command_3);

        if !Command::new("sh")
            .arg("-c")
            .arg(&command_3)
            .status()?
            .success()
        {
            fs::remove_dir_all(&temporary_repo)?;
            fs::remove_dir_all(&out_repo)?;
            return Err(PortapakError::CommandUnsuccessful(command_3));
        }
        fs::remove_dir_all(&temporary_repo)?;

        Ok(out_repo)
    }

    pub fn set_appid(&mut self, metadata: String) -> Result<(), PortapakError> {
        for line in metadata.lines() {
            if line.starts_with("name=") || line.starts_with("Name=") {
                self.appid = line.split_once("=").unwrap().1.to_string();
                return Ok(());
            }
        }
        Err(PortapakError::CommandUnsuccessful(
            "Could not find app name= in metadata".to_string(),
        ))
    }

    pub fn run_self(&self, from_repo: Option<PathBuf>) -> Result<(), PortapakError> {
        if let Some(repo) = from_repo {
            let command = format!(
                "flatpak --env=HOME={} --app-path={} run {}",
                env::var("HOME").unwrap(),
                repo.as_os_str().to_string_lossy(),
                &self.appid
            );
            log::debug!("{}", command);
            if Command::new("sh")
                .arg("-c")
                .arg(&command)
                .env("HOME", self.config.get_home_directory())
                .status()?
                .success()
            {
                Ok(())
            } else {
                Err(PortapakError::CommandUnsuccessful(command))
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
                Err(PortapakError::CommandUnsuccessful(format!(
                    "flatpak run {}",
                    self.appid
                )))
            }
        }
    }
}

pub fn run_flatpak(mut flatpak: Flatpak, config: UserConfig) -> Result<(), PortapakError> {
    let tmp = config.get_temporary_dir();
    let (tmp_dir, out_dir) = (tmp.join("tmp"), tmp.join("out"));
    let repo = flatpak.extract_repo(tmp_dir, out_dir)?;
    flatpak.set_appid(fs::read_to_string(repo.join("metadata"))?)?;
    let res = flatpak.run_self(Some(repo.join("files")));
    fs::remove_dir_all(repo)?;
    res
}
