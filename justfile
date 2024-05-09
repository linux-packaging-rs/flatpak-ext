name := 'portapak'

rootdir := ''
prefix := '/usr'

base-dir := absolute_path(clean(rootdir / prefix))

bin-src := 'target' / 'release' / name
bin-dst := base-dir / 'bin' / name

desktop-src := 'data' / 'io.ryanabx.Portapak.desktop'
desktop-dst := base-dir / 'share' / 'applications' / 'io.ryanabx.Portapak.desktop'

icon-src := 'data' / 'io.ryanabx.Portapak.svg'
icon-dst := base-dir / 'share' / 'icons' / 'hicolor' / 'scalable' / 'apps' / 'io.ryanabx.Portapak.svg'

build *args:
    cargo build --release {{args}}

install:
    install -Dm0755 {{bin-src}} {{bin-dst}}
    install -Dm0644 {{desktop-src}} {{desktop-dst}}
    install -Dm0644 {{icon-src}} {{icon-dst}}

uninstall:
    rm -f {{bin-dst}}
    rm -f {{desktop-dst}}
    rm -f {{icon-dst}}