# Ambra — project context

This file is the shared context for anyone working on Ambra: people and AI coding agents.
Read it before changing anything. Keep it short and up to date.

## What we are building

Ambra is a free, open-source (MIT) slicer for MSLA / LCD resin 3D printers.

- **Design** feels like LycheeSlicer: dark, clean, modern, a big viewport, a clear
  step-by-step workflow (Prepare → Supports → Slice & Export).
  We take the *feel*, not the assets: no copied icons, images, texts or layouts pixel-for-pixel.
- **Features** match LycheeSlicer and Chitubox combined, with every feature available in the GUI.
- **Editing** works like Blender: Object mode and Edit mode, G/R/S hotkeys, axis locks,
  modifiers, full undo/redo.
- **Terminal**: everything the GUI can do, the CLI can do too. A user must be able to work
  with Ambra using only the terminal.
- **No ads, no accounts, no telemetry, no paywalls. Ever.**

## The three editions

| Binary | What it is | Targets |
|---|---|---|
| `ambra` | Full GUI | Windows, Linux |
| `ambra-lite` | Simple, fast GUI with the same features | Linux first (Windows too, cheap to add) |
| `ambra-cli` | Command line, interactive shell and TUI | Everywhere, ships inside both packages |

**Ambra Lite rules:** same engine and same features as the full app. Only the presentation is
cut down: no animations, no fancy materials, no heavy effects, simple panels like mslicer.
It must run on an old laptop with integrated graphics, on OpenGL 2.1 / GLES 2, with software
rendering (Mesa llvmpipe) as a fallback, and on a system without a desktop
(from a bare TTY through the `cage` kiosk compositor, or as a TUI).

## Packaging targets

- **Windows:** installer (`.msi`) and portable `.zip`.
- **Linux:** AppImage, `.deb`, `.rpm`, AUR (`ambra-bin`, `ambra-lite-bin`, `ambra-git`), Flatpak, `.tar.gz`.
- Both `ambra` and `ambra-lite` get every Linux format.

## Presets

- Printers: as many as we can, with their native file formats (`.goo`, `.ctb`, Photon Workshop formats, …).
- Resins: popular resins with their real properties and exposure settings.
- See `docs/FORMATS.md` and `docs/PRESETS.md`.

## Tech stack (decided)

- **Language:** Rust (stable). One Cargo workspace.
- **GUI:** `egui` + `eframe`. Full app renders with `wgpu`; Lite renders with `glow` (OpenGL).
- **CLI:** `clap`; TUI with `ratatui`.
- **Details:** `docs/ARCHITECTURE.md`.

## Ground rules

1. **One action system.** Every user action is a `Command` in `ambra-core`. GUI buttons, hotkeys,
   the CLI, the built-in console and undo/redo all go through it. No feature lives only in the GUI.
2. **Engine first.** Logic lives in library crates. GUI crates only draw and dispatch commands.
3. **Permissive dependencies only:** MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0 (as a library).
   No GPL/AGPL code or crates. Checked by `cargo deny`.
4. **Clean-room.** Do not copy code from GPL/AGPL projects (mslicer, UVtools, PrusaSlicer) or from
   closed slicers (Lychee, Chitubox). Reading public docs about file formats is fine.
   Testing our output with those tools is fine.
5. **Never decompile, unpack or reverse-engineer LycheeSlicer or Chitubox binaries**, and never
   extract their preset databases. Lychee is studied only as a black box: by using it, by its
   public docs, changelogs and videos, and by looking at the output files it produces from our own
   test models. We copy *results and behaviour goals*, never code or data. See `docs/RESEARCH.md`.
6. **Presets are our own data**, compiled from manufacturer specs, manufacturer exposure charts and
   real test prints, with a source for every value. Resin settings are personal (printer, color,
   batch, temperature), so the calibration wizard is the main way to get them, not the database.
7. **Performance matters.** Big models (millions of triangles) and 16K screens are the normal case.
8. **Tests:** every file format has round-trip tests and a golden sample. Slicing has image tests.
9. **Languages:** UI in English and Russian from day one (`fluent`). Code, comments and commits in English.
10. **Commits and changelogs:** short, plain English, ready for GitHub.

## Releases

- Every version (each roadmap milestone and every fix release after it) gets its own commit(s),
  a git tag `vX.Y.Z` and a **GitHub Release** with a changelog.
- The changelog is plain English, short bullets, ready to paste, no "see README" pointers.
  Same text goes to `CHANGELOG.md`.
- CI attaches all packages to the release (see `docs/ARCHITECTURE.md` → Packaging).
- Versions before 1.0 are marked as pre-release.

## Test hardware

- Main test printer: **Elegoo Saturn 4 Ultra 16K** (`.goo` format).
- Main test resin: **Anycubic ABS-Like Resin Pro 2** (clear and grey).
  Known good settings on that printer: bottom exposure 20 s, normal exposure 1.5 s.
- Other printers are marked *experimental* until someone confirms a real print.

## Repository map

```
assets/            logo and the script that draws it
docs/ROADMAP.md    milestones and tasks
docs/ARCHITECTURE.md  crates, data flow, tech choices
docs/FORMATS.md    printer file formats and priorities
docs/PRESETS.md    printer and resin database format
docs/UI.md         layout of both apps and hotkeys
docs/FEATURES.md   feature checklist vs Lychee Lite / Plus and Chitubox
docs/RESEARCH.md   how we study Lychee and other slicers (black-box only)
docs/CALIBRATION.md  calibration wizard: finding exposure for your resin batch
CHANGELOG.md       release notes
```

Code does not exist yet. The planned workspace layout is in `docs/ARCHITECTURE.md`.
