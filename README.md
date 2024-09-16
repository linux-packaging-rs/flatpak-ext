Flatpak-ext is a binary that allows you to do extra things with flatpak, including:

> **NOTE (2024/09/16):** This project is currently being revived, which will take some time! I've learned a lot about rust since starting this project and I hope to clean it up a lot and add more features and better documentation.

* Running flatpaks without installing them **[Implemented]**
* More functionality coming soon!

## Examples

Run Inkscape from flathub without installing it:

```sh
flatpak-ext run-temp -a org.inkscape.Inkscape
```

## Contributing

There are many ways we can extend and automate flatpak to make it work in new exciting ways. Contributions are absolutely welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for more information!