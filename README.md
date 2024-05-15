![Portapak](res/social_preview.png)

It's as simple as running `portapak <path_to_flatpak>`!

> **NOTE:** Please help test it and [Submit ISSUES](https://github.com/ryanabx/portapak/issues/new) when you come across them!

## Build Requirements

**Libflatpak is required to build this project**

```sh
# Ubuntu, Debian, etc.
sudo apt-get install -y libflatpak-dev
# Fedora
sudo dnf install -y flatpak-devel
```

## Install/Uninstall

```sh
just build
just install
```

```sh
just uninstall
```

## Contributing

See [https://github.com/ryanabx/portapak/issues/1](https://github.com/ryanabx/portapak/issues/1) for ideas on future work!