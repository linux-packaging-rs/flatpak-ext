# Flatrun: Run flatpaks without installing them!

Flatrun is a command-line tool that lets you run a flatpak once, without it being installed to your system or user installation.

## Examples:

Run inkscape from flathub:

```shell
flatpak run io.github.ryanabx.flatrun -a org.inkscape.Inkscape
```

Run a locally downloaded flatpak bundle:

```shell
flatpak run io.github.ryanabx.flatrun -f ~/Documents/waycheck.flatpak
```

Get help for more commands:

```shell
flatpak run io.github.ryanabx.flatrun -h
```