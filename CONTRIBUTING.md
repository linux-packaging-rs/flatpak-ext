# Contributing

## Build Dependencies

* libflatpak-devel
* rust
* cargo

## Dependencies

* libflatpak

## Building flatpak-ext

```shell
git clone https://github.com/ryanabx/flatpak-ext
cargo build
```

## Flatpak-ext structure

Flatpak-ext is structured as both a lib and a bin, with the bin basically just calling the functions from the lib from a nice command-line interface.

The library is a collection of tools separated by modules, with a common error type and a few other common abstractions in the `types` module.

## License

Flatpak-ext is licensed under the MIT license, and no CLA is required to contribute.