# Roadmap

Each milestone ends with: commits → tag `vX.Y.Z` → **GitHub Release with a plain-English changelog**
and packages built by CI. Fix releases (`0.3.1`, …) follow the same steps.
Tasks are rough; they turn into GitHub issues when the milestone starts.
The full feature list with sources is in `docs/FEATURES.md`.

---

## Research track (runs alongside everything)

Black-box study of LycheeSlicer and Chitubox, rules in `docs/RESEARCH.md`.

- [ ] Build the research model set (`tests/samples/research/`)
- [ ] Record Lychee auto supports on each model (Lite, then Plus trial): counts, sizes, spacing, rafts
- [ ] Record hollowing, drain holes, orientation and arrange results
- [ ] Record UI flow screenshots for each workflow step → design notes for `docs/UI.md`
- [ ] Collect manufacturer exposure charts (Elegoo, Anycubic, Siraya Tech, Phrozen, …)
- [ ] Read public papers on support generation and hollowing

---

## Distribution track

- [x] Release files: Windows installer + portable exe, deb, rpm, AppImage, Flatpak bundle, PKGBUILD (0.0.2)
- [ ] AUR: publish `copal-bin` and `copal-lite-bin` (needs an AUR account and an SSH key secret)
- [ ] apt repository on GitHub Pages, signed with a project GPG key → `sudo apt install copal`
- [ ] Flathub: from-source manifest (vendored Cargo crates), screenshots, submission PR — once the
      app slices (Flathub does not accept placeholder apps)
- [ ] winget and Scoop manifests for Windows
- [ ] Code signing for the Windows installer (avoids the SmartScreen warning)

---

## 0.0 — Foundation ✅ released

- [x] Cargo workspace with all crates from `docs/ARCHITECTURE.md` (stubs for later ones)
- [x] GitHub Actions: build + test on Linux and Windows, `cargo fmt`, `clippy`, `cargo deny`
- [x] Release workflow: on tag, build everything, create a GitHub Release, attach packages,
      take the changelog from `CHANGELOG.md`
- [x] `Command` trait, `Document`, undo/redo stack in `copal-core`
- [x] Text command language shared by CLI, console and scripts
- [x] `.copal` project file v1 (save / load)
- [x] `copal-cli`: `new`, `info`, `run`, `script`, `commands`, `--json`
- [x] Preview apps: amber theme with crystal borders, simple 3D view, inspector, console
- [x] Issue templates: bug, feature, printer test report, resin test report

---

## 0.1 — It slices ✅ released

- [x] Mesh import: STL (binary + ASCII), OBJ, 3MF
- [x] Transforms: move, rotate, scale, mirror, duplicate, drop to plate, arrange
- [x] CPU slicer: plane cut → oriented segments → non-zero winding scanline, multithreaded
- [x] Anti-aliasing (sub-rows + exact coverage), grey levels
- [x] Bottom layers, transition layers, wait times, two-stage lift / retract
- [x] `.goo` writer + reader, checked against files from another slicer that printed well
- [x] Thumbnails from a tiny software renderer
- [x] Preset library compiled in from `presets/`, personal presets (resin, exposure, motion,
      supports, hollow, layout)
- [x] Print history with project snapshots, ratings and notes
- [x] OpenGL 3D view (GL 2.1 / GLES 2 / GL 3 core), perspective camera, picking
- [x] Move / rotate / scale gizmos with adjustable size, drag models over the plate
- [x] Layout / Prepare / Export steps, settings tabs, resizable panels, layer preview
- [x] Background export with progress and cancel
- [x] `copal-shell`: one set of commands for the console and `copal-cli shell`
- [x] CLI: import, export, history, presets, printers, resins, inspect
- [x] **Confirm image orientation** of `.goo` files against Lychee with the F test model (0.2)
- [ ] A test print on the Saturn 4 Ultra 16K
- [ ] Benchmarks in CI

**Done when:** a model sliced by Copal prints correctly on the Saturn 4 Ultra 16K.

---

## 0.2 — Supports, hollowing, drain holes (released)

Supports moved ahead of orientation: the owner needs them to print a complex figure part.

- [x] Auto supports: light / medium / heavy presets, settings from the Supports tab
- [x] Modes: Auto (islands + overhangs) and Islands only
- [x] The tilted XYZ cube holds (regression test), every island of the Armadillo test model supported
- [x] Support geometry: tips, bodies, bends around the model, feet on the model, braces, raft
- [x] Auto bracing (spanning tree of neighbours, zigzag braces)
- [x] Manual supports: click to add one, it braces itself to its neighbours when the option is on
- [x] Select, resize and delete supports; supports move with their model
- [x] Island detection from the layers, shown in the 3D view
- [x] Hollowing 3D (voxel distance field + surface nets), wall thickness, precision
- [x] Drain holes: click to place, size and depth, real holes in the slices
- [x] CLI and console: `supports.*`, `holes.*`, `object.hollow`, `supports`, `islands`, `hollow`
- [x] Side panels fold away like IntelliJ tool windows (Alt+1/2/3)
- [x] Personal resins: form in the Resin tab, import, resin folder watched while the app runs
- [x] `.goo` image orientation confirmed against Lychee

---

## 0.3 — Orientation, calibration, more prepare tools

- [ ] Auto orientation: score rotations by overhang area, islands and peel area
- [ ] Lay flat on a picked face
- [ ] Islands in the layer preview
- [ ] Resin trap / suction cup check for hollow models
- [ ] Exposure test generator + "pick the best chip" + save personal resin
      (`docs/CALIBRATION.md`), in both apps and the CLI
- [ ] Selection filters (tips, bases, braces), drag a support tip to move it
- [ ] Proximity check, STL / OBJ export with supports, tree supports
- [ ] More printers from manufacturers' public specs
- [ ] `.ctb` writer + reader (unencrypted first)
- [ ] English + Russian UI (`fluent`)
- [ ] Guide: running Lite from a TTY with `cage`

**Done when:** auto supports print the research model set on the main printer without failures,
and match or beat the recorded Lychee results.

---

## 0.4 — Polish of the full app

- [ ] Nicer material (resin look), outlines, shadows on the plate
- [ ] Undo/redo history panel, macro recording → `.copalscript`
- [ ] Editable hotkey map
- [ ] Autosave and local backups
- [ ] Reload model when its file changes on disk
- [ ] Picture view (nice render to share)
- [ ] Full calibration wizard UI

---

## 0.5 — Preparation tools

- [ ] Mesh repair: holes, flipped normals, non-manifold edges, self-intersections
- [ ] Fill the plate with copies
- [ ] Multi-plate projects
- [ ] "Magic button" (one model) and "batch button" (many models): orient → support → arrange
- [ ] Hollowing 2D, 3D and voxel; hollow blockers (manual + automatic); infill / lattice
- [ ] Internal supports for hollow models
- [ ] Drain holes: place, size, auto suggest
- [ ] XY compensation: elephant foot, hole compensation
- [ ] Measuring tool, physical size mode, layer thickness preview
- [ ] Print time, resin volume, weight and cost estimate
- [ ] Problem report panel (islands, traps, thin walls, out of bounds)

---

## 0.6 — Blender-style editing

- [ ] Object mode / Edit mode (Tab)
- [ ] Select vertices, edges, faces; box, lasso, linked, loop selection
- [ ] G / R / S with X / Y / Z axis lock and typed values
- [ ] Extrude, inset, delete, merge, fill, subdivide, smooth, decimate
- [ ] Boolean mode: union, difference, intersection (Manifold)
- [ ] Planar cut, split tool, assembly keys / pins
- [ ] Ironing (smooth a chosen surface)
- [ ] Primitives and text / logo emboss
- [ ] Non-destructive modifiers: mirror, array, boolean, decimate, hollow
- [ ] Simple sculpt brushes (inflate, smooth, flatten) — stretch goal

---

## 0.7 — Database and formats

- [ ] Tier 2 file formats from `docs/FORMATS.md`, writers and readers
- [ ] Encrypted `.ctb` versions
- [ ] 30+ printer presets from manufacturer specs
- [ ] 50+ resin starting points from manufacturer exposure charts
- [ ] Preset browser with search and filters
- [ ] Community flow: printer test reports and resin calibration results → presets marked "verified"

---

## 0.8 — Pro features

- [ ] GPU slicing (`wgpu` compute), checked pixel-by-pixel against CPU
- [ ] Send to printer over Wi-Fi (SDCP first, NanoDLP next)
- [ ] XY gauge, bottom exposure test, temperature notes in the calibration wizard
- [ ] Per-layer settings, multi-exposure, variable layer height
- [ ] Open, edit and convert existing sliced files
- [ ] `copal-cli tui` with layer preview in the terminal (sixel / kitty graphics)

---

## 1.0 — Stable

- [ ] Bug bash, performance pass, memory pass
- [ ] User guide (English + Russian)
- [ ] Flathub, AUR, winget submissions
- [ ] Project file format frozen with migrations

---

## Later ideas

- macOS builds
- Plugin / scripting API (Lua or Rhai)
- Multi-material and the LUMEN format
- More UI languages
