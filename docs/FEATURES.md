# Feature checklist

Goal: everything that LycheeSlicer **Lite + Plus** and **Chitubox** offer, free, in every edition.
The Lychee list comes from its public pricing page (Lychee Slicer 8, October 2026).
"Milestone" points to `docs/ROADMAP.md`.

Legend: **L** = Lychee Lite (free), **P** = Lychee Plus (paid), **C** = Chitubox, **A** = Copal extra.

## General

| Feature | From | Milestone |
|---|---|---|
| Core slicing for resin printers | L | 0.1 |
| Multi-plate projects (several build plates in one project) | L limited, P full | 0.5 |
| "Magic button": one click → orient, support, arrange, ready to print | L basic | 0.5 |
| "Batch button": the same for many models at once | P | 0.5 |
| Network / Wi-Fi printing | P | 0.8 |
| Local backups and autosave (no cloud, no account) | L | 0.4 |
| Multi-language UI | L | 0.2 (EN, RU) |
| Works fully offline, no ads, no account | A | always |
| Full CLI, shell, scripts, macros | A | 0.1 → 0.4 |

## Supports

| Feature | From | Milestone |
|---|---|---|
| Auto supports | L, P (new system), C | 0.3 |
| Island detector | L, C | 0.3 |
| Rafts | L | 0.3 |
| Premium raft styles | P | 0.3 |
| Parenting and bracing (supports linked to each other, cross braces) | L | 0.3 |
| Inline supports | P | 0.3 |
| Proximity detection (supports too close to the model) | P | 0.3 |
| Grid supports | P | 0.3 |
| Projection supports | P | 0.3 |
| Support painting (paint areas that need / must not get supports) | P | 0.3 |
| Internal supports for hollow models | P (Lychee 8) | 0.5 |
| Manual add / move / delete / resize | L, C | 0.3 |
| Suction cup and resin trap detection | C | 0.3 |

## Hollowing and layout

| Feature | From | Milestone |
|---|---|---|
| Hollowing 2D | L | 0.5 |
| Hollowing 3D | P | 0.5 |
| Voxel hollowing 3D | P | 0.5 |
| Hollow blockers (areas kept solid), automatic blockers | P | 0.5 |
| Infill / lattice inside hollow parts | P (Lychee 8), C | 0.5 |
| Drain holes | L, C | 0.5 |
| Repair tool | L, C | 0.5 |
| Auto orientation | L (Lychee 8), C | 0.5 |
| Auto arrange | L, C | 0.5 |
| Boolean mode | P | 0.6 |
| Ironing (smoothing a surface) | P | 0.6 |
| Measuring tool | P, C | 0.5 |
| Planar cut | P, C | 0.6 |
| Split tool | P, C | 0.6 |
| Assembly keys (pins for cut parts) | P | 0.6 |
| Manual / automatic model update (reload changed file from disk) | L manual, P auto | 0.4 |
| Text and logo emboss | C | 0.6 |
| Blender-style edit mode, modifiers | A | 0.6 |

## Preview

| Feature | From | Milestone |
|---|---|---|
| Layer preview | L, C | 0.2 |
| Layer thickness preview | L | 0.5 |
| Physical size mode (see the model at real size on screen) | P | 0.5 |
| Picture view (nice render for sharing) | L | 0.4 |
| Problem report (islands, traps, thin walls) | C | 0.5 |

## Import and export

| Feature | From | Milestone |
|---|---|---|
| STL / OBJ / 3MF import | L, C | 0.1 |
| STL / OBJ export with supports | L | 0.3 |
| Export scene | L | 0.4 |
| Native printer formats (`.goo`, `.ctb`, …) | L, C | 0.1 → 0.7 |
| Open and edit already sliced files | C | 0.8 |

## Slicing settings

| Feature | From | Milestone |
|---|---|---|
| Anti-aliasing, grey levels, image blur | L, C | 0.1 |
| Bottom / transition layers, wait times | L, C | 0.1 |
| Two-stage lift and retract (TSMC) | L, C | 0.1 |
| XY compensation, elephant foot | L, C | 0.5 |
| Exposure test print (RERF-style) | C | 0.2 |
| Calibration wizard per resin batch | A | 0.2 |

Not planned: model library / store, AI model generation, cloud accounts.
