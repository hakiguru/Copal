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

## 0.0 — Foundation

- [ ] Cargo workspace with all crates from `docs/ARCHITECTURE.md` (empty stubs)
- [ ] GitHub Actions: build + test on Linux and Windows, `cargo fmt`, `clippy`, `cargo deny`
- [ ] Release workflow: on tag, build everything, create a GitHub Release, attach packages,
      take the changelog from `CHANGELOG.md`
- [ ] `Command` trait, `Document`, undo/redo stack in `ambra-core`
- [ ] `.ambra` project file v1 (save / load)
- [ ] Issue templates: bug, feature, printer test report, resin test report

**Done when:** CI is green on both OSes, a project file round-trips, a test release appears on GitHub.

---

## 0.1 — Engine MVP (terminal only)

- [ ] Mesh import: STL (binary + ASCII), OBJ, 3MF
- [ ] Transforms: move, rotate, scale, drop to plate, center
- [ ] CPU slicer: plane intersection → polygons → raster, multithreaded
- [ ] Anti-aliasing (supersampling), grey levels, image blur
- [ ] Bottom layers, transition layers, wait times, two-stage lift / retract
- [ ] `.goo` writer + reader
- [ ] `.ctb` writer + reader (start with the unencrypted version)
- [ ] Thumbnails rendered by a tiny software renderer
- [ ] Preset loader + first presets: Saturn 4 Ultra 16K, ABS-Like Pro 2 (clear, grey)
- [ ] `ambra-cli slice`, `ambra-cli info <file>`, PNG layer export
- [ ] Benchmarks: slicing time and memory on a 16K screen

**Done when:** a model sliced by `ambra-cli` prints correctly on the Saturn 4 Ultra 16K.

---

## 0.2 — Ambra Lite + calibration

- [ ] `ambra-ui-common`: theme, 3D viewport, build plate grid
- [ ] Lite renderer on `glow`, tested on OpenGL 2.1 and llvmpipe
- [ ] Load models, select, move/rotate/scale with gizmos and number fields
- [ ] Printer and resin pickers
- [ ] Slice with progress and cancel; layer preview slider
- [ ] Export dialog
- [ ] **Exposure test generator + "pick the best chip" + save personal resin preset**
      (`docs/CALIBRATION.md`), in Lite and CLI
- [ ] English + Russian UI
- [ ] Packaging: AppImage, `.deb`, `.rpm`, AUR, `.tar.gz` for `ambra-lite` and `ambra-cli`
- [ ] Guide: running Lite from a TTY with `cage`

**Done when:** a user on a weak laptop with integrated graphics calibrates a resin, slices and exports.

---

## 0.3 — Supports

- [ ] Research spike results written up (goals from the Lychee study + papers)
- [ ] Overhang and island detection (mesh + layer based), shown in the viewport
- [ ] Auto supports: light / medium / heavy, editable parameters
- [ ] Support geometry: tips, shafts, joints, bases, branches
- [ ] Parenting and bracing between supports
- [ ] Support types: inline, grid, projection
- [ ] Support painting: areas that need supports / must stay clean
- [ ] Proximity detection (supports too close to the model)
- [ ] Rafts: flat, chamfered, skate-style and more
- [ ] Manual editing: add, delete, move tip, resize, select many
- [ ] Suction cup / resin trap detection
- [ ] STL / OBJ export with supports
- [ ] CLI: `supports.*` commands

**Done when:** auto supports print the research model set on the main printer without failures,
and match or beat the recorded Lychee results.

---

## 0.4 — Ambra, the full app

- [ ] Layout from `docs/UI.md`: top bar, workflow tabs, tool shelf, inspector, layer timeline
- [ ] `wgpu` renderer: resin-like material, SSAO, outlines, selection highlight
- [ ] Undo/redo history panel
- [ ] Built-in console panel (same command language as the CLI), macro recording → `.ambrascript`
- [ ] Hotkey map (Blender-like by default, editable)
- [ ] Autosave and local backups
- [ ] Reload model when its file changes on disk
- [ ] Export scene, picture view (nice render to share)
- [ ] Full calibration wizard UI
- [ ] Windows: `.msi` installer and portable zip
- [ ] Full app packages for every Linux format, Flatpak manifests for both editions

**Done when:** import → supports → slice → export is comfortable in the full app on Windows and Linux.

---

## 0.5 — Preparation tools

- [ ] Mesh repair: holes, flipped normals, non-manifold edges, self-intersections
- [ ] Auto orientation (fewer islands, less support, smaller layer area)
- [ ] Auto arrange, fill plate with copies
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
- [ ] `ambra-cli tui` with layer preview in the terminal (sixel / kitty graphics)

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
