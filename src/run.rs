use crate::config::{FlatpakHandle, RunConfig};

pub fn run_flatpak(flatpak: FlatpakHandle, config: RunConfig) -> Result<(), ()> {
    let appid = flatpak.to_appid();
    
    Ok(())
}
