build-flatpak:
    flatpak install -y --or-update flathub org.flatpak.Builder.BaseApp
    flatpak install -y --or-update flathub org.freedesktop.Sdk.Extension.rust-stable/x86_64/23.08
    flatpak run org.flatpak.Builder --force-clean --install --user .flatpak-target flatpak/io.github.ryanabx.flatrun.yml
    rm -r .flatpak-target