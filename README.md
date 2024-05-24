# Flatrun - Run Flatpaks without installing!

Run the graphical user interface:

```sh
flatpak run io.github.ryanabx.flatrun --gui
```

Or, run a .flatpak bundle straight from the terminal!:

```sh
flatpak run io.github.ryanabx.flatrun bundle [PATH]
```

> **NOTE:** Please help test this and [Submit ISSUES](https://github.com/ryanabx/flatrun/issues/new) when you come across them!

## Building/Installing

**Flatpak is the only supported method of building Flatrun**. Install `flatpak-builder` and run:

```sh
flatpak-builder --install --user [BUILD_DIR] flatpak/io.github.ryanabx.flatrun.yml
```

## Screenshots

![Loading screen for Flatrun](res/screenshot1.png)

![Running screen for Flatrun](res/screenshot2.png)