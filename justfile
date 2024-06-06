build-flatpak:
    python3 tools/generate-sources.py Cargo.lock
    rm -f flatpak/generated-sources.json
    mv generated-sources.json flatpak/generated-sources.json
    flatpak install -y --or-update flathub org.flatpak.Builder.BaseApp
    flatpak install -y --or-update flathub org.freedesktop.Sdk.Extension.rust-stable/x86_64/23.08
    flatpak run org.flatpak.Builder --force-clean --install --user .flatpak-target flatpak/io.github.ryanabx.flatrun.yml
    rm -r .flatpak-target