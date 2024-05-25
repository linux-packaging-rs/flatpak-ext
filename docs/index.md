# Flatrun: https://github.com/ryanabx/flatrun

Run Flatpaks without installing them!

## Installing

> **NOTE:** The main distribution method will be from `flathub`. The review process is currently occurring at https://github.com/flathub/flathub/pull/5280.

```sh
flatpak install -y flathub io.github.ryanabx.flatrun
```

## Building

**Flatpak is the only supported method of building Flatrun**. Install `flatpak-builder` and run:

```sh
flatpak-builder --install --user [BUILD_DIR] flatpak/io.github.ryanabx.flatrun.yml
```

## Issues

[Submit issues](https://github.com/ryanabx/flatrun/issues/new)

## Command Reference

```sh
flatpak run io.github.ryanabx.flatrun -h
```