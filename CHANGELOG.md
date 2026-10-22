# Changelog

All notable changes to Copal. Newest first.
Each version here is also published as a GitHub Release.

## 0.0.2

- Installers! Windows setup (.exe) and portable .exe files
- Linux packages: .deb (Ubuntu, Debian, Mint), .rpm (Fedora, openSUSE), AppImage, Flatpak bundle, PKGBUILD for Arch
- Start menu / app menu shortcuts for Copal and Copal Lite, and .copal files open in Copal
- The crystal icon is built into the Windows .exe files
- The console looks more like a terminal: the input line follows the last output, the prompt and the text line up, and a click anywhere in the console focuses it
- First presets: Elegoo Saturn 4 Ultra 16K and Anycubic ABS-Like Pro 2 Grey with tested settings
- Notes from studying Lychee Slicer: supports, hollowing, orientation and the .goo files it writes

## 0.0.1

- New default theme: calm dark or light colors with simple orange outlines
- Light mode in white and grey tones
- The amber crystal look is now the Art theme
- Settings window (Settings button, Ctrl+,) to switch theme and dark / light mode; the choice is saved
- `settings` command in the console and in copal-cli, e.g. `settings theme=art mode=light`
- The console looks and works like a terminal: `copal:project$` prompt, ↑ / ↓ history, Tab completion
- Releases are now regular releases, so the newest one shows up as "Latest" on GitHub

## 0.0.0

The foundation. Copal can't slice yet, but the engine, the apps and the release pipeline are in place.

- First code: a Rust workspace with the engine crates and three apps (Copal, Copal Lite, Copal CLI)
- Command system with undo and redo: every change is a command, the same for buttons, console and terminal
- Text command language, e.g. `object.move id=1 x=10`, with clear error messages
- `.copal` project files: save and open, safe writing, version check
- Copal CLI: `new`, `info`, `run`, `script` and `commands`, with `--json` output
- Copal and Copal Lite preview apps: amber theme with crystal borders, simple 3D view of the build plate, object inspector, scene list, built-in console, Ctrl+Z / Ctrl+Shift+Z
- Test objects (cubes) to try moving, rotating, scaling and dropping to the plate
- Automatic checks on every push (Linux and Windows) and license checks
- Automatic releases: a new version in Cargo.toml plus a CHANGELOG section is all it takes
- Issue forms for bugs, ideas, printer tests and resin results
- Project setup: logo and wordmark, README, MIT license, planning docs and the wiki
