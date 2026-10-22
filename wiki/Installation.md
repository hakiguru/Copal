Get the files from the [latest release](https://github.com/hakiguru/Copal/releases/latest).
Every release has packages for Windows and the popular Linux systems.

**Full or Lite?** Full packages contain **Copal**, **Copal Lite** and the terminal tool
**copal-cli**. Packages with **lite** in the name contain only Copal Lite and copal-cli — for old
laptops and small systems. Install one or the other, not both. See [[Which edition?|Which-Edition]].

## Windows

**Installer:** download `Copal-Setup-<version>.exe` and run it. You get Start menu shortcuts for
Copal and Copal Lite, and `.copal` files open in Copal. Options during setup: a desktop shortcut
and adding `copal-cli` to PATH. You can install for yourself only (no admin rights needed) or for
all users.

**Portable:** `Copal-<version>-portable.exe` (or `Copal-Lite-<version>-portable.exe`) — one file,
nothing to install, just run it. `copal-<version>-windows-x86_64.zip` has all three programs.

> Windows may show "Windows protected your PC" because the installer is not signed yet.
> Click **More info → Run anyway**.

## Ubuntu, Debian, Linux Mint, Pop!_OS

```sh
sudo apt install ./copal_<version>_amd64.deb
# or the light version:
sudo apt install ./copal-lite_<version>_amd64.deb
```

Remove with `sudo apt remove copal`.

## Fedora, openSUSE

```sh
sudo dnf install ./copal-<version>-1.x86_64.rpm        # Fedora
sudo zypper install ./copal-<version>-1.x86_64.rpm     # openSUSE
```

## Arch, Manjaro, EndeavourOS

Download `copal-bin.PKGBUILD` (or `copal-lite-bin.PKGBUILD`), then:

```sh
mkdir copal && cd copal
mv ~/Downloads/copal-bin.PKGBUILD PKGBUILD
makepkg -si
```

The package will also be published on the AUR, then `yay -S copal-bin` will work.

## Any Linux: AppImage

```sh
chmod +x Copal-<version>-x86_64.AppImage
./Copal-<version>-x86_64.AppImage
```

`Copal-Lite-<version>-x86_64.AppImage` starts Copal Lite.

## Any Linux: Flatpak

```sh
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user copal-<version>.flatpak
flatpak run io.github.hakiguru.Copal                          # Copal
flatpak run --command=copal-lite io.github.hakiguru.Copal     # Copal Lite
```

Copal will be submitted to Flathub once it can slice; then it appears in GNOME Software and KDE Discover.

## Any Linux: by hand

`copal-<version>-linux-x86_64.tar.gz` — unpack anywhere and run `bin/copal`, `bin/copal-lite` or
`bin/copal-cli`.

## Updating

Install the new version the same way, it replaces the old one. An apt repository (so
`sudo apt upgrade` brings updates) and Flathub are planned.

## Uninstalling

Use your package manager (Windows: *Settings → Apps → Copal → Uninstall*). Settings stay in
`~/.config/copal` (Linux) or `%APPDATA%\Copal` (Windows) — delete that folder to remove them too.
