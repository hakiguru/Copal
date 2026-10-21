# Changelog

All notable changes to Copal. Newest first.
Each version here is also published as a GitHub Release.

## 0.0.0

The foundation. Copal can't slice yet, but the engine, the apps and the release pipeline are in place.

- First code: a Rust workspace with the engine crates and three apps (Copal, Copal Lite, Copal CLI)
- Command system with undo and redo: every change is a command, the same for buttons, console and terminal
- Text command language, e.g. `object.move id=1 x=10`, with clear error messages
- `.copal` project files: save and open, safe writing, version check
- Copal CLI: `new`, `info`, `run`, `script` and `commands`, with `--json` output
- Copal and Copal Lite preview apps: amber theme with crystal borders, simple 3D view of the build plate, object inspector, scene list, built-in console, Ctrl+Z / Ctrl+Shift+Z
- Test objects (cubes) to try moving, rotating, scaling and dropping to the plate
- Automatic checks on every push (Linux and Windows), automatic releases from tags, license checks
- Issue forms for bugs, ideas, printer tests and resin results
- Project setup: logo and wordmark, README, MIT license, planning docs and the wiki
