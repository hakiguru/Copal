# Architecture

Built parts are described as they are; planned parts say so. Change this file when reality disagrees.

## Big picture

```
            ┌──────────────┐  ┌───────────────┐  ┌──────────────────────────┐
            │    copal     │  │  copal-lite   │  │        copal-cli         │
            │ full GUI     │  │ simple GUI    │  │ commands · shell · TUI   │
            │ egui + glow  │  │ egui + glow   │  │ clap                     │
            └──────┬───────┘  └──────┬────────┘  └────────────┬─────────────┘
                   │   copal-ui-common (look, 3D view,        │
                   │   gizmos, panels, file browser)          │
                   └──────────────┬───────────────────────────┤
                                  │        copal-shell (console / shell commands,
                                  │        export, presets, history)
                                  ▼
                                  ▼
                    ┌──────────────────────────┐
                    │        copal-core         │  project, scene, Command system,
                    │                           │  undo/redo, settings, jobs
                    └────────────┬──────────────┘
         ┌──────────┬────────────┼────────────┬────────────┬────────────┐
         ▼          ▼            ▼            ▼            ▼            ▼
    copal-mesh  copal-supports copal-slice copal-formats copal-presets copal-net
    io, repair, auto/manual    CPU + GPU   .goo .ctb …   printers,     send to
    booleans,   supports,      rasterizer  readers and   resins        printer
    hollowing   islands                    writers
```

## Crates

| Crate | Job |
|---|---|
| `copal-core` | Project file, objects, transforms, project settings (print, supports, hollow, layout). **Command system**, undo/redo, app settings, print history (`prints.rs`), the preset library the commands use. |
| `copal-mesh` | Mesh type (indexed triangles), import/export (STL, OBJ, 3MF, PLY), repair (holes, normals, non-manifold), booleans, plane cut, decimation, hollowing (voxel/SDF offset), drain holes, edit-mode operations. |
| `copal-supports` | Overhang and island detection, support point sampling, auto supports (light/medium/heavy, tree/branch), manual editing, rafts, collision checks. |
| `copal-slice` | Turns a scene into layer images. CPU rasterizer (scanline, multithreaded with `rayon`) and GPU path (`wgpu` compute). Anti-aliasing, blur, XY compensation (elephant foot, holes). |
| `copal-formats` | One module per printer format: writer first, reader second. Thumbnails, layer encoding (RLE etc.), encryption where needed. |
| `copal-presets` | Loads the printer and resin database (`presets/`), user presets, validation. |
| `copal-net` | Sending files to printers over the network (SDCP for Elegoo/Chitu boards first). |
| `copal-shell` | The shell both GUIs' consoles and `copal-cli shell` run: help, undo/redo, save/open, import, export, presets, print history, completion. |
| `copal-ui-common` | Shared egui widgets, theme, OpenGL 3D view (`render.rs`, the only module allowed `unsafe`), camera, gizmos, panels, file browser. |
| `copal` | Full app binary. |
| `copal-lite` | Lite app binary. |
| `copal-cli` | CLI binary: one-shot commands, interactive shell, script runner, TUI. |

Layout:

```
crates/copal-core/
crates/copal-mesh/
crates/copal-supports/
crates/copal-slice/
crates/copal-formats/
crates/copal-presets/
crates/copal-net/
crates/copal-shell/
crates/copal-ui-common/
apps/copal/
apps/copal-lite/
apps/copal-cli/
presets/printers/<brand>/<model>.toml
presets/resins/<brand>/<resin>.toml
packaging/  (deb, rpm, AUR, AppImage, Flatpak, Windows)
tests/samples/  (models and golden output files)
```

## The Command system

The key idea: **one way to do anything**.

```rust
// Sketch, not final.
pub trait Command {
    fn name(&self) -> &'static str;          // "supports.auto", "object.rotate", ...
    fn apply(&self, doc: &mut Document) -> Result<Undo>;
}
```

- Every command has a stable name and serializable arguments (`serde`).
- GUI buttons and hotkeys build commands. The CLI parses the same commands from text.
- Undo/redo stores the inverse for each applied command.
- The built-in console in the GUI, the `copal-cli shell` REPL and script files (`.copalscript`)
  all speak the same command language. A user can record a macro in the GUI and run it from the terminal.

## CLI shape

```sh
# One shot
copal-cli slice model.stl --printer elegoo/saturn-4-ultra-16k \
  --resin anycubic/abs-like-pro-2-grey --orient auto --supports medium -o model.goo

# Step by step with a project file
copal-cli new part.copal --printer elegoo/saturn-4-ultra-16k
copal-cli add part.copal model.stl
copal-cli run part.copal supports.auto density=heavy
copal-cli export part.copal -o part.goo

# Interactive
copal-cli shell part.copal     # REPL with autocomplete
copal-cli tui part.copal       # text UI with layer preview in the terminal
```

`--json` on every command for machine-readable output. Exit codes are documented.

## Project file `.copal`

A zip archive:

```
project.json        scene, objects, transforms, settings, printer and resin ids
meshes/<id>.bin     meshes in a compact binary form
supports/<id>.json  support structures
```

`project.json` has a `version`: 1 = objects only, 2 = adds `settings` (print, supports, hollow,
layout). Older files open with default settings. Newer files are refused with a clear message.
Supports (0.3) will be stored per object, in the object's own frame, so they move with it.

## Print history

Every export makes a folder in `<data dir>/history/` (`~/.local/share/copal`, `%APPDATA%\Copal`,
or `COPAL_DATA_DIR`): `record.json` (file, printer, resin, settings, estimates, rating, outcome,
note), `project.copal` (the project exactly as sliced) and `preview.png`. Opening a snapshot
creates an unsaved copy, so the snapshot never changes.

## Presets

Built-in presets are the TOML files in `presets/`, embedded at build time by
`crates/copal-presets/build.rs`. Personal presets live in `<config dir>/presets/<kind>/<name>.toml`
with kinds `resins`, `exposure`, `motion`, `supports`, `hollow`, `layout`. Loading a preset runs one
`<section>.set …` command with every value, so it is one undo step and can be replayed as text.

## Rendering

Both apps use the same OpenGL renderer (`crates/copal-ui-common/src/render.rs`) inside an egui
paint callback. Shaders are written for GLSL 1.20 / ES 1.00 and 1.40 / ES 3.00 at once, so the
renderer runs on OpenGL 2.1, OpenGL 3 core, GLES 2/3 and Mesa llvmpipe (no GPU).

- Meshes are uploaded once per mesh (16 bytes per vertex: position + packed face normal, flat
  shading) and freed when no object uses them.
- Two-sided lighting, overhang colouring in the fragment shader, plate / grid / volume as lines.
- Picking is a CPU ray cast (bounding box first). Gizmos are drawn with egui on top of the 3D view.
- No multisampling is requested: some drivers have no such config and the app would not start.

Planned: a nicer material and effects in the full app (0.4), level of detail for huge meshes.

**Lite on a system without a desktop:** run inside `cage` (a tiny Wayland kiosk compositor)
straight from a TTY: `cage copal-lite`. Without any graphics, use `copal-cli shell`.

## Slicing

Built (`crates/copal-slice`):

1. Bake transforms; mirrored objects get their triangles flipped back.
2. Triangles go into 0.5 mm height buckets. For each layer, cut at the middle of the layer: each
   crossing triangle gives one segment, oriented from its downward edge to its upward edge, so outer
   contours run counter-clockwise and holes clockwise.
3. Scanline fill with the **non-zero winding rule** (overlapping models merge, inverted inner walls
   make holes). Anti-aliasing takes N sub-rows per pixel row and exact horizontal coverage, summed
   with a difference buffer; only the columns a row touches are processed.
4. Rows stream into the format encoder (`RowSink`), so a 16K layer never sits in memory whole.
   Layers are rendered in parallel batches with `rayon` and written in order.
5. The header gets the real resin volume (from the lit pixels) and print time at the end.

Benchmark (2 CPU cores): a 60 mm sphere, 2 M triangles, 2000 layers at 16K with 8× anti-aliasing
→ 45 s; the volume matches the analytic value to 0.1 ml.

## Auto supports (outline)

1. Detect what needs support: downward faces past the overhang angle, local minima (islands),
   checked again on sliced layers.
2. Sample support points by area and angle; density from the preset (light/medium/heavy).
3. Route each point down to the plate or to the model, avoiding collisions (BVH ray tests).
4. Merge nearby supports into branches (tree style) to save resin.
5. Generate geometry: tip (contact sphere/cone), shaft, joints, base, raft.
6. Re-check islands on the final sliced layers and patch gaps.

This is the hardest part of the project. It gets its own research spike before coding.

## Libraries (all permissive)

| Area | Crate |
|---|---|
| Math | `glam` |
| Spatial queries, BVH | `parry3d` |
| Booleans | `manifold3d` (Manifold, Apache-2.0) |
| Simplification | `meshopt` |
| Mesh import | `stl_io`, `tobj`, own 3MF reader |
| Parallelism | `rayon` |
| GUI | `egui`, `eframe`, `egui-wgpu`, `egui_glow` |
| GPU | `wgpu` |
| CLI / TUI | `clap`, `ratatui`, `reedline` (shell) |
| Data | `serde`, `toml`, `serde_json` |
| Compression / images | `zstd`, `flate2`, `png` |
| Translations | `fluent` |
| License check | `cargo-deny` |

## Packaging

| Target | Tool |
|---|---|
| Windows `.msi` | `cargo-wix` |
| Windows portable | zip from CI |
| `.deb` | `cargo-deb` |
| `.rpm` | `cargo-generate-rpm` |
| AppImage | `linuxdeploy` + `appimagetool` |
| AUR | PKGBUILDs in `packaging/aur/`, published by CI |
| Flatpak | manifest in `packaging/flatpak/`, later on Flathub |

All builds run in GitHub Actions on every tag. Each Linux format is produced twice:
`copal` and `copal-lite`. `copal-cli` is inside both and also ships alone as a `.tar.gz`.
