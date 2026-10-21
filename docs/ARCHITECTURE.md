# Architecture

This is the planned design. Nothing here is built yet. Change it when reality disagrees.

## Big picture

```
            ┌──────────────┐  ┌───────────────┐  ┌──────────────────────────┐
            │    copal     │  │  copal-lite   │  │        copal-cli         │
            │ full GUI     │  │ simple GUI    │  │ commands · shell · TUI   │
            │ egui + wgpu  │  │ egui + glow   │  │ clap · ratatui           │
            └──────┬───────┘  └──────┬────────┘  └────────────┬─────────────┘
                   │   copal-ui-common (widgets, viewport,    │
                   │   hotkeys, i18n)                          │
                   └──────────────┬────────────────────────────┘
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
| `copal-core` | Project file, scene graph, objects, transforms, settings. **Command system**, undo/redo, background jobs with progress and cancel. |
| `copal-mesh` | Mesh type (indexed triangles), import/export (STL, OBJ, 3MF, PLY), repair (holes, normals, non-manifold), booleans, plane cut, decimation, hollowing (voxel/SDF offset), drain holes, edit-mode operations. |
| `copal-supports` | Overhang and island detection, support point sampling, auto supports (light/medium/heavy, tree/branch), manual editing, rafts, collision checks. |
| `copal-slice` | Turns a scene into layer images. CPU rasterizer (scanline, multithreaded with `rayon`) and GPU path (`wgpu` compute). Anti-aliasing, blur, XY compensation (elephant foot, holes). |
| `copal-formats` | One module per printer format: writer first, reader second. Thumbnails, layer encoding (RLE etc.), encryption where needed. |
| `copal-presets` | Loads the printer and resin database (`presets/`), user presets, validation. |
| `copal-net` | Sending files to printers over the network (SDCP for Elegoo/Chitu boards first). |
| `copal-ui-common` | Shared egui widgets, theme, 3D viewport, camera, gizmos, hotkey map, translations. |
| `copal` | Full app binary. |
| `copal-lite` | Lite app binary. |
| `copal-cli` | CLI binary: one-shot commands, interactive shell, script runner, TUI. |

Planned layout:

```
crates/copal-core/
crates/copal-mesh/
crates/copal-supports/
crates/copal-slice/
crates/copal-formats/
crates/copal-presets/
crates/copal-net/
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
thumbnail.png
```

Versioned, with migrations. Open in all three editions.

## Rendering

| | Full (`copal`) | Lite (`copal-lite`) |
|---|---|---|
| Backend | `wgpu`: Vulkan, DX12, OpenGL fallback | `glow`: OpenGL 2.1 / GLES 2+ |
| No GPU | — | Mesa llvmpipe (software) |
| Shading | PBR-like resin look, SSAO, outlines | Flat/Lambert, no effects |
| Big meshes | GPU buffers, LOD | aggressive LOD, decimated preview |
| Animations | Yes | None |

**Lite on a system without a desktop:** run inside `cage` (a tiny Wayland kiosk compositor)
straight from a TTY: `cage copal-lite`. If there is no graphics at all, use `copal-cli tui`.

## Slicing

1. Bake transforms, merge model + supports + raft.
2. For each layer height `z`: intersect triangles with the plane → closed polygons.
3. Rasterize polygons to the printer's pixel grid (even-odd fill), with N× supersampling for anti-aliasing.
4. Post-process: blur, XY compensation, bottom/transition layers.
5. Encode each layer for the target format and stream to disk (never hold all 16K layers in RAM).

CPU path is the reference and always works. GPU path must produce identical images (tested).

## Auto supports (outline)

1. Detect what needs support: downward faces past the overhang angle, local minima (islands),
   checked again on sliced layers.
2. Sample support points by area and angle; density from the preset (light/medium/heavy).
3. Route each point down to the plate or to the model, avoiding collisions (BVH ray tests).
4. Merge nearby supports into branches (tree style) to save resin.
5. Generate geometry: tip (contact sphere/cone), shaft, joints, base, raft.
6. Re-check islands on the final sliced layers and patch gaps.

This is the hardest part of the project. It gets its own research spike before coding.

## Libraries (planned, all permissive)

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
