> [!NOTE]
> 🚧 This page describes a planned feature. It arrives in **Copal 0.0**. Details may change.


## Requirements
- Rust (stable) — install with [rustup](https://rustup.rs)
- Linux: `build-essential`, `cmake`, `pkg-config`, `libxkbcommon-dev`, `libwayland-dev`
- Windows: Visual Studio Build Tools (C++)

## Build

```sh
git clone https://github.com/hakiguru/copal
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

## Packages
`cargo deb`, `cargo generate-rpm`, AppImage and Flatpak scripts live in `packaging/`.
CI builds all of them on every tag.
