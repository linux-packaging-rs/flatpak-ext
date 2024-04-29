use std::{fs, path::PathBuf};

use crate::{flatpak::Flatpak, PortapakError};

pub enum StorageType {
    Network,
    MMC,
    SDCard,
    HardDisk,
}

impl ToString for StorageType {
    fn to_string(&self) -> String {
        match self {
            Self::Network => "network".to_string(),
            Self::MMC => "mmc".to_string(),
            Self::SDCard => "sdcard".to_string(),
            Self::HardDisk => "harddisk".to_string(),
        }
    }
}

pub struct FlatpakRepo {
    name: String,
    path: PathBuf,
    display_name: String,
    storage_type: StorageType,
}

impl ToString for FlatpakRepo {
    fn to_string(&self) -> String {
        format!(
            "[Installation \"{}\"\nPath={}\nDisplayName={}\nStorageType={}",
            self.name,
            self.path.as_os_str().to_string_lossy(),
            self.display_name,
            self.storage_type.to_string()
        )
    }
}

impl FlatpakRepo {
    pub fn new(name: String, path: PathBuf) -> Self {
        Self {
            name: name.clone(),
            path,
            display_name: name,
            storage_type: StorageType::HardDisk,
        }
    }
    pub fn install_self(&self, installations_dir: PathBuf) -> Result<(), PortapakError> {
        // ensure config directory has been made
        let _ = fs::create_dir_all(&installations_dir);
        // ensure flatpak repo path has been made
        let _ = fs::create_dir_all(&self.path);
        fs::write(self.conf_dir(installations_dir), self.to_string())?;
        Ok(())
    }

    pub fn remove_self(&self, installations_dir: PathBuf) -> Result<(), PortapakError> {
        // remove config
        let _ = fs::remove_file(self.conf_dir(installations_dir));
        // remove repo
        let _ = fs::remove_dir_all(&self.path);
        Ok(())
    }

    fn conf_dir(&self, installations_dir: PathBuf) -> PathBuf {
        installations_dir.join(&format!("_portapak_{}.conf", self.name))
    }

    pub fn install_flatpak(&self, flatpak: Flatpak) -> Result<String, PortapakError> {
        if !flatpak.path.exists() {
            return Err(PortapakError::FileNotFound(flatpak.path));
        }
        Ok("".to_string())
    }
}
