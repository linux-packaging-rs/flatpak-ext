name := 'flatrun'
agent-name := 'flatrun-agent'
export APPID := 'io.github.ryanabx.flatrun'

rootdir := ''
prefix := '/usr'
flatpak-prefix := '/app'

base-dir := absolute_path(clean(rootdir / prefix))
flatpak-base-dir := absolute_path(clean(rootdir / flatpak-prefix))

export INSTALL_DIR := base-dir / 'share'

bin-src := 'target' / 'release' / name
flatpak-bin-dst := flatpak-base-dir / 'bin' / name

agent-bin-src := 'target' / 'release' / agent-name
flatpak-agent-bin-dst := flatpak-base-dir / 'libexec' / agent-name

desktop := APPID + '.desktop'
desktop-src := 'data' / desktop
flatpak-desktop-dst := flatpak-base-dir / 'share' / 'applications' / desktop

bundle-desktop := APPID + '-bundle.desktop'
bundle-desktop-src := 'data' / bundle-desktop
flatpak-bundle-desktop-dst := flatpak-base-dir / 'share' / 'applications' / bundle-desktop

metainfo := APPID + '.metainfo.xml'
metainfo-src := 'data' / metainfo
flatpak-metainfo-dst := flatpak-base-dir / 'share' / 'metainfo' / metainfo

icons-src := 'data' / 'flatrun512.png'
flatpak-icons-dst := flatpak-base-dir / 'share' / 'icons' / 'hicolor' / '512x512' / 'apps' / 'io.github.ryanabx.flatrun.png'

# Default recipe which runs `just build-release`
default: build-release

# Runs `cargo clean`
clean:
    cargo clean

# Removes vendored dependencies
clean-vendor:
    rm -rf .cargo vendor vendor.tar

# `cargo clean` and removes vendored dependencies
clean-dist: clean clean-vendor

# Compiles with debug profile
build-debug *args:
    cargo build {{args}}

# Compiles with release profile
build-release *args: (build-debug '--release' args)

# Compiles release profile with vendored dependencies
build-vendored *args: vendor-extract (build-release '--frozen --offline' args)

build-flatpak *args:
    touch /app/lib/libflatpak.so
    touch /app/bin/flatpak
    install -Dm0644 data/flatpak/flatpak.pc /app/lib/pkgconfig/flatpak.pc
    cargo --offline fetch --manifest-path Cargo.toml --verbose
    cargo --offline build --release --verbose
    rm /app/lib/pkgconfig/flatpak.pc
    rm /app/lib/libflatpak.so
    rm /app/bin/flatpak

# Runs a clippy check
check *args:
    cargo clippy --all-features {{args}} -- -W clippy::pedantic

# Runs a clippy check with JSON message format
check-json: (check '--message-format=json')

dev *args:
    cargo fmt
    just run {{args}}

# Run with debug logs
run *args:
    env RUST_LOG=flatrun=info RUST_BACKTRACE=full cargo run --release {{args}}

# Installs files
flatpak:
    install -Dm0755 {{bin-src}} {{flatpak-bin-dst}}
    install -Dm0755 {{agent-bin-src}} {{flatpak-agent-bin-dst}}
    install -Dm0644 {{desktop-src}} {{flatpak-desktop-dst}}
    install -Dm0644 {{bundle-desktop-src}} {{flatpak-bundle-desktop-dst}}
    install -Dm0644 {{metainfo-src}} {{flatpak-metainfo-dst}}
    install -Dm0644 {{icons-src}} {{flatpak-icons-dst}}

# Vendor dependencies locally
vendor:
    #!/usr/bin/env bash
    mkdir -p .cargo
    cargo vendor --sync Cargo.toml | head -n -1 > .cargo/config.toml
    echo 'directory = "vendor"' >> .cargo/config.toml
    echo >> .cargo/config.toml
    echo '[env]' >> .cargo/config.toml
    if [ -n "${SOURCE_DATE_EPOCH}" ]
    then
        source_date="$(date -d "@${SOURCE_DATE_EPOCH}" "+%Y-%m-%d")"
        echo "VERGEN_GIT_COMMIT_DATE = \"${source_date}\"" >> .cargo/config.toml
    fi
    if [ -n "${SOURCE_GIT_HASH}" ]
    then
        echo "VERGEN_GIT_SHA = \"${SOURCE_GIT_HASH}\"" >> .cargo/config.toml
    fi
    tar pcf vendor.tar .cargo vendor
    rm -rf .cargo vendor

# Extracts vendored dependencies
vendor-extract:
    rm -rf vendor
    tar pxf vendor.tar