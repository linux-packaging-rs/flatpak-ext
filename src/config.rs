use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub global: RunConfig,
    pub overrides: HashMap<FlatpakHandle, RunConfig>,
    pub nogui: Option<bool>,
    pub tmp_dir: Option<PathBuf>,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            global: RunConfig {
                home_dir: None,
                desktop_integration: Some(true),
            },
            overrides: HashMap::new(),
            nogui: None,
            tmp_dir: None,
        }
    }
}

impl UserConfig {
    pub fn get_temporary_dir(&self) -> PathBuf {
        self.tmp_dir.clone().unwrap_or(
            Path::new(&format!(
                "{}/portapak/",
                env::var("RUNTIME_DIR").unwrap_or(format!(
                    "/run/user/{}",
                    env::var("UID").unwrap_or("1000".to_string())
                ))
            ))
            .to_path_buf(),
        )
    }

    pub fn get_config(&self, handle: FlatpakHandle) -> RunConfig {
        match handle.clone() {
            FlatpakHandle::Appid(id) => {
                if let Some(ac) = self.overrides.get(&FlatpakHandle::Appid(id)) {
                    ac.blend(self.global.clone())
                } else {
                    self.global.clone()
                }
            }
            FlatpakHandle::Path(path) => {
                if let Some(ac) = self.overrides.get(&FlatpakHandle::Path(path)) {
                    ac.blend(self.global.clone())
                } else if let Some(ac) =
                    self.overrides.get(&FlatpakHandle::Appid(handle.to_appid()))
                {
                    ac.blend(self.global.clone())
                } else {
                    self.global.clone()
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FlatpakHandle {
    Appid(String),
    Path(PathBuf),
}

impl FlatpakHandle {
    pub fn to_appid(&self) -> String {
        match self {
            Self::Appid(id) => id.into(),
            Self::Path(path) => path.file_stem().unwrap().to_string_lossy().into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    home_dir: Option<PathBuf>,
    desktop_integration: Option<bool>,
}

impl RunConfig {
    pub fn get_home_directory(&self) -> PathBuf {
        self.home_dir
            .clone()
            .unwrap_or(Path::new(&env::var("HOME").unwrap()).to_path_buf())
    }

    pub fn blend(&self, other: Self) -> Self {
        Self {
            home_dir: if self.home_dir.is_some() {
                self.home_dir.clone()
            } else {
                other.home_dir
            },
            desktop_integration: if self.desktop_integration.is_some() {
                self.desktop_integration
            } else {
                other.desktop_integration
            },
        }
    }
}
