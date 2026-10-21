> [!NOTE]
> 🚧 This page describes a planned feature. It arrives in **Copal 0.2**. Details may change.


**Preview builds (0.0):** download `copal-<version>-linux-x86_64.tar.gz` or
`copal-<version>-windows-x86_64.zip` from [Releases](https://github.com/hakiguru/Copal/releases), unpack,
and run `copal`, `copal-lite` or `copal-cli`. The packages below arrive with 0.2.

Copal will ship for **Windows** and **Linux**. Every Linux format exists for both
**Copal** and **Copal Lite**. The command line tool `copal-cli` is included in both.

## Windows

- **Installer** (`.msi`) — download from [Releases](https://github.com/hakiguru/Copal/releases), run, done.
- **Portable** (`.zip`) — unpack anywhere and run `copal.exe`. Nothing is installed.

## Linux

| Format | Distros | How |
|---|---|---|
| AppImage | any | download, `chmod +x Copal-*.AppImage`, run |
| `.deb` | Debian, Ubuntu, Mint, Pop!_OS | `sudo apt install ./copal_*.deb` |
| `.rpm` | Fedora, openSUSE | `sudo dnf install ./copal-*.rpm` |
| AUR | Arch, Manjaro, EndeavourOS | `yay -S copal-bin` or `yay -S copal-lite-bin` |
| Flatpak | any | `flatpak install flathub io.github.hakiguru.Copal` *(after Flathub approval)* |
| `.tar.gz` | any | unpack and run the binary |

## Updating

Packages from apt, dnf, AUR and Flatpak update with your system.
AppImage and portable builds: download the new version from [Releases](https://github.com/hakiguru/Copal/releases).

## Uninstalling

Remove the package with your package manager. Your settings and personal presets stay in
`~/.config/copal` (Linux) or `%APPDATA%\Copal` (Windows) — delete that folder to remove them too.
