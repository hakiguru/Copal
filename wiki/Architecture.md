A short tour. The full design is in
[docs/ARCHITECTURE.md](https://github.com/hakiguru/Copal/blob/main/docs/ARCHITECTURE.md).

- **One engine, three faces.** `copal`, `copal-lite` and `copal-cli` share all logic.
- **Everything is a command.** Buttons, hotkeys, the console, scripts and undo/redo all use the
  same command system, so nothing is GUI-only.
- **Crates:** `copal-core` (project, commands), `copal-mesh` (geometry), `copal-supports`,
  `copal-slice`, `copal-formats`, `copal-presets`, `copal-net`, `copal-ui-common`.
- **Rust + egui.** The full app renders with wgpu, Lite with OpenGL.
- **Permissive licenses only.** All dependencies are MIT/Apache-compatible.
