1. All work for the version is merged into `main`.
2. `CHANGELOG.md` gets a section for the version, in plain English.
3. A tag `vX.Y.Z` is pushed.
4. CI builds every package (Windows, AppImage, deb, rpm, AUR, Flatpak, tar.gz) for Copal and
   Copal Lite and creates a **GitHub Release** with the changelog and all files.
5. Versions before 1.0 are marked as pre-release.

Fix releases (`0.3.1`) follow the same steps.
