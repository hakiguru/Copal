# Copal — project context

This file is the shared context for anyone working on Copal: people and AI coding agents.
Read it before changing anything. Keep it short and up to date.

## What we are building

Copal is a free, open-source (MIT) slicer for MSLA / LCD resin 3D printers.

- **Design** feels like LycheeSlicer: dark, clean, modern, a big viewport, a clear
  step-by-step workflow (Layout → Prepare → Export).
  We take the *feel*, not the assets: no copied icons, images, texts or layouts pixel-for-pixel.
- **Features** match LycheeSlicer and Chitubox combined, with every feature available in the GUI.
- **Editing** works like Blender: Object mode and Edit mode, G/R/S hotkeys, axis locks,
  modifiers, full undo/redo.
- **Terminal**: everything the GUI can do, the CLI can do too. A user must be able to work
  with Copal using only the terminal.
- **No ads, no accounts, no telemetry, no paywalls. Ever.**

## The three editions

| Binary | What it is | Targets |
|---|---|---|
| `copal` | Full GUI | Windows, Linux |
| `copal-lite` | Simple, fast GUI with the same features | Linux first (Windows too, cheap to add) |
| `copal-cli` | Command line, interactive shell and TUI | Everywhere, ships inside both packages |

**Copal Lite rules:** same engine and same features as the full app. Only the presentation is
cut down: no animations, no fancy materials, no heavy effects, simple panels like mslicer.
It must run on an old laptop with integrated graphics, on OpenGL 2.1 / GLES 2, with software
rendering (Mesa llvmpipe) as a fallback, and on a system without a desktop
(from a bare TTY through the `cage` kiosk compositor, or as a TUI).

## Packaging targets

- **Windows:** installer (`.msi`) and portable `.zip`.
- **Linux:** AppImage, `.deb`, `.rpm`, AUR (`copal-bin`, `copal-lite-bin`, `copal-git`), Flatpak, `.tar.gz`.
- Both `copal` and `copal-lite` get every Linux format.

## Presets

- Printers: as many as we can, with their native file formats (`.goo`, `.ctb`, Photon Workshop formats, …).
- Resins: popular resins with their real properties and exposure settings.
- See `docs/FORMATS.md` and `docs/PRESETS.md`.

## Tech stack (decided)

- **Language:** Rust (stable). One Cargo workspace.
- **GUI:** `egui` + `eframe` with `glow`. Both apps share one OpenGL renderer that works on
  OpenGL 2.1 / GLES 2 / GL 3 core and Mesa llvmpipe.
- **CLI:** `clap`. The shell (`copal-shell`) is shared by `copal-cli shell` and the GUI consoles.
- **Details:** `docs/ARCHITECTURE.md`.

## Ground rules

1. **One action system.** Every user action is a `Command` in `copal-core`. GUI buttons, hotkeys,
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
    The author is the project owner only. **Never add AI co-author or session trailers**
    (`Co-Authored-By: …`, `Claude-Session: …` and similar) to commits, PRs or releases.

## Releases

- Every version (each roadmap milestone and every fix release after it) gets its own commit(s),
  a git tag `vX.Y.Z` and a **GitHub Release** with a changelog.
- To release: bump `version` in the root `Cargo.toml`, add a `## X.Y.Z` section to `CHANGELOG.md`,
  push to `main`. `.github/workflows/release.yml` builds, tags and publishes by itself.
- The changelog is plain English, short bullets, ready to paste, no "see README" pointers.
  Same text goes to `CHANGELOG.md`.
- CI attaches all packages to the release (see `docs/ARCHITECTURE.md` → Packaging).
- Releases are normal (not pre-release), so GitHub shows the newest one as "Latest".

## Test hardware

- Main test printer: **Elegoo Saturn 4 Ultra 16K** (`.goo` format).
- Main test resin: **Anycubic ABS-Like Resin Pro 2** (clear and grey).
  Known good settings on that printer (from the owner's printed files): 0.03 mm layers,
  1.3–1.4 s exposure, 5 bottom layers × 29 s, 5 transition layers, two-stage lift.
  See `presets/` and `docs/research/lychee-2026-10.md`.
- Other printers are marked *experimental* until someone confirms a real print.

## Repository map

```
assets/            logo, the script that draws it, test-models/ (our own small test models)
docs/ROADMAP.md    milestones and tasks
docs/ARCHITECTURE.md  crates, data flow, tech choices
docs/FORMATS.md    printer file formats and priorities
docs/PRESETS.md    printer and resin database format
docs/UI.md         layout of both apps and hotkeys
docs/FEATURES.md   feature checklist vs Lychee Lite / Plus and Chitubox
docs/RESEARCH.md   how we study Lychee and other slicers (black-box only)
docs/CALIBRATION.md  calibration wizard: finding exposure for your resin batch
CHANGELOG.md       release notes
presets/           printer and resin presets (TOML)
packaging/         installers: Linux (deb, rpm, AppImage, AUR, Flatpak) and Windows (Inno Setup)
docs/research/     black-box notes about other slicers
wiki/              wiki pages (published automatically)
```

The user guide is the GitHub wiki. Edit pages in `wiki/` in this repo — a GitHub Action
(`.github/workflows/wiki.yml`) publishes them to the wiki on every push to `main`.
When a feature lands, update its wiki page and remove the "planned" banner.

```
crates/copal-core       document, Command system, undo/redo, text language, .copal files,
                        project settings, print history, app settings
crates/copal-mesh       triangle mesh, STL/OBJ/3MF import, binary mesh format, ray casts
crates/copal-presets    printer/resin database (embedded from presets/), settings structs,
                        personal presets
crates/copal-slice      layer rasterizer (non-zero winding, anti-aliasing, streaming rows)
crates/copal-formats    .goo writer/reader, thumbnails, export of a project
crates/copal-shell      shell commands shared by the GUI consoles and copal-cli shell
crates/copal-ui-common  theme, crystal borders, widgets, OpenGL 3D view, gizmos, panels
crates/copal-*          other engine crates (stubs until their milestone)
apps/copal              full app        apps/copal-lite   Lite        apps/copal-cli   CLI
tools/ui-shot.sh        screenshot an app in a virtual display (Xvfb) to check the UI
```

## Working on the code

- Before a commit: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`. CI runs the same plus `cargo deny`.
- UI changes: check them with `tools/ui-shot.sh` (add `--interact` to click and type).
- `unsafe` is denied everywhere except the OpenGL renderer (`copal-ui-common/src/render.rs`).
- Slicing crates are optimized in debug builds too (see `[profile.dev.package…]` in `Cargo.toml`).
- The `.goo` image orientation for printers with `mirror_y` is not confirmed yet: don't call the
  format stable before the F test model (`assets/test-models/`) matches another slicer's output.
- Two themes, each dark or light: **Default** (neutral, simple orange outlines) and **Art** (amber
  crystal borders). Widgets never hard-code colors: take them from `Look::pal` (`palette.rs`)
  and draw outlines with `Look::outline`, which picks the right style. Default is what users see first.
