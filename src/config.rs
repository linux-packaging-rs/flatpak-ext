use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Clone)]
pub struct UserConfig {
    pub global: RunConfig,
    pub overrides: HashMap<FlatpakHandle, RunConfig>,
    pub nogui: Option<bool>,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct RunConfig {
    HomeDir: Option<PathBuf>,
    DesktopIntegration: Option<bool>,
}
