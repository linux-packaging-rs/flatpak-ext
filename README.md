# Flatrun - Run Flatpaks without installing!

Run a .flatpak bundle from a file:

```sh
flatpak run io.github.ryanabx.flatrun -f [PATH]
```

Run a flatpak straight from Flathub:

```sh
flatpak run io.github.ryanabx.flatrun -a [APPID]
```

For example, inkscape?



> **NOTE:** Please help test this and [Submit ISSUES](https://github.com/ryanabx/flatrun/issues/new) when you come across them!

## Building/Installing

**Flatpak is the only supported method of building Flatrun**. Install `flatpak-builder` and run:

Build requirements: just and flatpak-builder installed through flatpak

```sh
flatpak install org.flatpak.Builder
just build-flatpak
```

## Screenshots

![Loading screen for Flatrun](res/screenshot1.png)

![Running screen for Flatrun](res/screenshot2.png)

## Contributing

This project is open to fixes and new features! It'd be helpful to make an issue describing what you plan to implement to avoid duplicate work!