
---

### How to install

| System | File | How |
|---|---|---|
| **Windows** | `Copal-Setup-*.exe` | Run it. Start menu shortcuts for Copal and Copal Lite, `.copal` files open in Copal. |
| Windows, no install | `Copal-*-portable.exe` | Just run it. `Copal-Lite-*-portable.exe` for weak computers. |
| **Ubuntu, Debian, Mint** | `copal_*_amd64.deb` | `sudo apt install ./copal_*_amd64.deb` |
| **Fedora, openSUSE** | `copal-*.x86_64.rpm` | `sudo dnf install ./copal-*.x86_64.rpm` |
| **Arch, Manjaro** | `copal-bin.PKGBUILD` | Save as `PKGBUILD` in an empty folder, run `makepkg -si` |
| **Any Linux** | `Copal-*-x86_64.AppImage` | `chmod +x Copal-*.AppImage`, then run it |
| **Any Linux, Flatpak** | `copal-*.flatpak` | `flatpak install --user copal-*.flatpak` (needs the Flathub remote) |
| Any Linux, by hand | `copal-*-linux-x86_64.tar.gz` | Unpack and run `bin/copal` |

Files with **lite** in the name contain only Copal Lite and copal-cli — for old laptops with
integrated graphics. Full packages already include Copal Lite.
