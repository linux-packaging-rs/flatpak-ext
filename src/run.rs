use std::fs;

use crate::{config::UserConfig, flatpak::Flatpak, PortapakError};

pub fn run_flatpak(mut flatpak: Flatpak, config: UserConfig) -> Result<(), PortapakError> {
    let tmp = config.get_temporary_dir();
    let (tmp_dir, out_dir) = (
        tmp.join(format!("tmp/{}", flatpak.appid)),
        tmp.join(format!("out/{}", flatpak.appid)),
    );
    fs::create_dir_all(&tmp_dir)?;
    fs::create_dir_all(&out_dir)?;
    fs::remove_dir(&out_dir)?;
    flatpak.extract_repo(tmp_dir.clone(), out_dir.clone())?;
    fs::remove_dir_all(&tmp_dir)?;
    flatpak.set_appid(fs::read_to_string(out_dir.join("metadata"))?)?;
    let res = flatpak.run_self(Some(out_dir.join("files")));
    fs::remove_dir_all(&out_dir)?;
    res
}
