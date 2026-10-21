> [!NOTE]
> 🚧 This page describes a planned feature. It arrives in **Copal 0.2**. Details may change.


Copal will ship for **Windows** and **Linux**. Every Linux format exists for both
**Copal** and **Copal Lite**. The command line tool `copal-cli` is included in both.

## Windows

- **Installer** (`.msi`) — download from [Releases](https://github.com/hakiguru/copal/releases), run, done.
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
AppImage and portable builds: download the new version from [Releases](https://github.com/hakiguru/copal/releases).

## Uninstalling

Remove the package with your package manager. Your settings and personal presets stay in
`~/.config/copal` (Linux) or `%APPDATA%\Copal` (Windows) — delete that folder to remove them too.
