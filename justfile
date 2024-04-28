name := 'portapak'

rootdir := ''
prefix := '/usr'

base-dir := absolute_path(clean(rootdir / prefix))

bin-src := 'target' / 'release' / name
bin-dst := base-dir / 'bin' / name

desktop-src := 'data' / 'net.ryanabx.Portapak.desktop'
desktop-src := base-dir / 'share' / 'applications' / 'net.ryanabx.Portapak.desktop'

build *args:
    cargo build --release {{args}}

install:
    install -Dm0755 {{bin-src}} {{bin-dst}}
    install -Dm0644 {{desktop-src}} {{desktop-dst}}

uninstall:
    rm -f {{bin-dst}}
    rm -f {{desktop-dst}}