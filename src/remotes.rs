use libflatpak::{prelude::RemoteExt, Remote};

use crate::PortapakError;

pub fn flathub_remote() -> Result<Remote, PortapakError> {
    let flathub = Remote::new("flathub");
    flathub.set_url("https://dl.flathub.org/repo/");
    flathub.set_homepage("https://flathub.org");
    flathub.set_comment("Central repository of Flatpak applications");
    flathub.set_description("Central repository of Flatpak applications");
    flathub.set_icon("https://dl.flathub.org/repo/logo.svg");
    // TODO: Get binary gpg key
    // flathub.set_gpg_key({{KEY}});
    flathub.set_gpg_verify(false);
    Ok(flathub)
}
