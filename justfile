build-flatpak:
    python3 tools/generate-sources.py Cargo.lock
    rm -f flatpak/generated-sources.json
    mv generated-sources.json flatpak/generated-sources.json
    flatpak run org.flatpak.Builder --force-clean --install --user .flatpak-target flatpak/io.github.ryanabx.flatrun.yml
    rm -r .flatpak-target