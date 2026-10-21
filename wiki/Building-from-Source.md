
## Requirements
- Rust (stable) — install with [rustup](https://rustup.rs)
- Linux: a C toolchain (`sudo apt install build-essential`). Graphics and window libraries are
  loaded at runtime, nothing else to install.
- Windows: Visual Studio Build Tools (C++)

## Build

```sh
git clone https://github.com/hakiguru/Copal
cd copal
cargo build --release -p copal        # full app
cargo build --release -p copal-lite   # Lite
cargo build --release -p copal-cli    # CLI
```

Binaries land in `target/release/`.

## Test

```sh
cargo test --workspace
```

## Checking the interface

`tools/ui-shot.sh target/debug/copal shot.png [project.copal] [--interact]` starts an app in a virtual
display and saves a screenshot. Needs `xvfb`, `imagemagick` and, for `--interact`, `xdotool`.

## Packages
`cargo deb`, `cargo generate-rpm`, AppImage and Flatpak scripts live in `packaging/`.
CI builds all of them on every tag.
