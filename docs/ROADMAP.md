# Roadmap

Each milestone ends with a tagged release and packages from CI.
Tasks are rough; they turn into GitHub issues when the milestone starts.

---

## 0.0 — Foundation

Set up the ground so everything after it is easy.

- [ ] Cargo workspace with all crates from `docs/ARCHITECTURE.md` (empty stubs)
- [ ] GitHub Actions: build + test on Linux and Windows, `cargo fmt`, `clippy`, `cargo deny`
- [ ] Release workflow skeleton (builds on tag, uploads artifacts)
- [ ] `Command` trait, `Document`, undo/redo stack in `ambra-core`
- [ ] `.ambra` project file v1 (save / load)
- [ ] Test samples folder: a few small STL models (own or CC0)
- [ ] Issue templates: bug, feature, printer test report

**Done when:** CI is green on both OSes and a project file round-trips.

---

## 0.1 — Engine MVP (terminal only)

Prove the core works on a real printer.

- [ ] Mesh import: STL (binary + ASCII), OBJ, 3MF
- [ ] Transforms: move, rotate, scale, drop to plate, center
- [ ] CPU slicer: plane intersection → polygons → raster, multithreaded
- [ ] Anti-aliasing (supersampling) and greyscale output
- [ ] Bottom layers, transition layers, wait times, lift / retract settings (incl. two-stage)
- [ ] `.goo` writer + reader
- [ ] `.ctb` writer + reader (start with the unencrypted version)
- [ ] Thumbnails rendered by a tiny software renderer
- [ ] Preset loader + first presets: Saturn 4 Ultra 16K, ABS-Like Pro 2 (clear, grey)
- [ ] `ambra-cli slice` and `ambra-cli info <file>`
- [ ] PNG layer export for debugging
- [ ] Benchmarks: slicing time and memory on a 16K screen

**Done when:** a model sliced by `ambra-cli` prints correctly on the Saturn 4 Ultra 16K.

---

## 0.2 — Ambra Lite, first GUI

Something people can actually use, early.

- [ ] `ambra-ui-common`: theme, 3D viewport (orbit, pan, zoom, view presets), build plate grid
- [ ] Lite renderer on `glow`, tested on OpenGL 2.1 and llvmpipe
- [ ] Load models, select, move/rotate/scale with gizmos and number fields
- [ ] Printer and resin pickers
- [ ] Slice with progress bar and cancel; layer preview slider
- [ ] Export dialog
- [ ] English + Russian UI
- [ ] Packaging: AppImage, `.deb`, `.rpm`, AUR, `.tar.gz` for `ambra-lite` and `ambra-cli`
- [ ] Guide: running Lite from a TTY with `cage`

**Done when:** a user on a weak laptop with integrated graphics slices and exports a model.

---

## 0.3 — Supports

The hardest and most important feature.

- [ ] Research spike: support point sampling, routing, tree merging. Write findings in `docs/`.
- [ ] Overhang and island detection (mesh + layer based), shown in the viewport
- [ ] Auto supports: light / medium / heavy presets, editable parameters
- [ ] Support geometry: tips, shafts, joints, bases, branches
- [ ] Rafts: flat, chamfered, skate-style
- [ ] Manual editing: add, delete, move tip, change size, select many
- [ ] Suction cup / resin trap detection
- [ ] Support presets saved per project and globally
- [ ] CLI: `supports.auto`, `supports.add`, `supports.remove`, …

**Done when:** auto supports print the test set (mini figures, flat plates, hollow parts)
on the main printer without failures.

---

## 0.4 — Ambra, the full app

The Lychee-like experience.

- [ ] Layout from `docs/UI.md`: top bar, workflow tabs, tool shelf, inspector, layer timeline
- [ ] `wgpu` renderer: resin-like material, SSAO, outlines, selection highlight
- [ ] Undo/redo history panel
- [ ] Built-in console panel (same command language as the CLI)
- [ ] Macro recording → `.ambrascript`
- [ ] Hotkey map (Blender-like by default, editable)
- [ ] Windows build: `.msi` installer and portable zip
- [ ] Full app packages for every Linux format
- [ ] Flatpak manifests for both editions

**Done when:** the whole flow (import → supports → slice → export) is comfortable in the full app
on Windows and Linux.

---

## 0.5 — Preparation tools

- [ ] Mesh repair: holes, flipped normals, non-manifold edges, self-intersections
- [ ] Auto orientation (fewer islands, less support, smaller layer area)
- [ ] Auto arrange and fill plate with copies
- [ ] Hollowing with wall thickness, optional inner lattice
- [ ] Drain holes: place, size, auto suggest
- [ ] XY compensation: elephant foot, hole compensation
- [ ] Print time, resin volume, weight and cost estimate
- [ ] Problem report panel (islands, traps, thin walls, out of bounds)

---

## 0.6 — Blender-style editing

- [ ] Object mode / Edit mode (Tab)
- [ ] Select vertices, edges, faces; box, lasso, linked, loop selection
- [ ] G / R / S with X / Y / Z axis lock and typed values
- [ ] Extrude, inset, delete, merge, fill, subdivide, smooth, decimate
- [ ] Booleans: union, difference, intersection (Manifold)
- [ ] Cut by plane, with optional connector keys/pins
- [ ] Primitives: cube, cylinder, sphere, cone, plate, text (emboss)
- [ ] Non-destructive modifiers: mirror, array, boolean, decimate, hollow
- [ ] Simple sculpt brushes (inflate, smooth, flatten) — stretch goal

---

## 0.7 — Database and formats

- [ ] Tier 2 file formats from `docs/FORMATS.md`, writers and readers
- [ ] Encrypted `.ctb` versions
- [ ] 30+ printer presets, 50+ resin presets with sources
- [ ] Preset browser with search and filters
- [ ] "Printer test report" flow: users confirm a printer works → status becomes stable

---

## 0.8 — Pro features

- [ ] GPU slicing (`wgpu` compute), checked pixel-by-pixel against CPU
- [ ] Send to printer over Wi-Fi (SDCP first, NanoDLP next)
- [ ] Calibration prints: exposure test, XY accuracy test
- [ ] Per-layer settings, multi-exposure, variable layer height
- [ ] Open and inspect existing sliced files; convert between formats
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
- Multi-material and LUMEN format support
- More languages for the UI
