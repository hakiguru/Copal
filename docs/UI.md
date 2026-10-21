# Interface

## Full app — layout

Inspired by the feel of LycheeSlicer: dark, calm, everything one click away.
All icons, art and wording are our own.

```
┌───────────────────────────────────────────────────────────────────────────────┐
│ ◆ AMBRA   File Edit View   │ Printer: Saturn 4 Ultra 16K ▾ │ Resin: ABS-Like ▾ │
├──────────┬────────────────────────────────────────────────────────┬───────────┤
│ PREPARE  │                                                        │ Inspector │
│ SUPPORTS │                                                        │           │
│ SLICE    │                    3D viewport                         │ object /  │
│──────────│                    build plate                         │ tool      │
│ tools    │                                                        │ settings  │
│ for the  │                                                        │           │
│ current  │                                                        │───────────│
│ step     │                                                        │ Scene     │
│          │                                                        │ objects   │
├──────────┴────────────────────────────────────────────────────────┴───────────┤
│ Console ▸ supports.auto density=medium                 │ ▮▮▮▮ layer slider ▮▮ │
└───────────────────────────────────────────────────────────────────────────────┘
```

- **Workflow tabs** on the left: Prepare → Supports → Slice & Export. The tool shelf below them
  changes with the tab.
- **Inspector** on the right: settings of the selected object or the active tool.
- **Scene list** under the inspector: objects, supports, rafts, modifiers.
- **Bottom bar:** console (collapsible) and the layer slider after slicing.
- **Theme:** dark by default, amber accent from the logo, light theme available.

## Ambra Lite — layout

Like mslicer: one window, plain panels, no animations.

```
┌──────────────────────────────────────────────────────────┐
│ File  Edit  Supports  Slice  Help                         │
├────────────────┬─────────────────────────────────────────┤
│ Printer  ▾     │                                          │
│ Resin    ▾     │            3D viewport                   │
│ Objects        │            (simple shading)              │
│ Transform      │                                          │
│ Supports       │                                          │
│ Slice  [Go]    │                                          │
├────────────────┴─────────────────────────────────────────┤
│ status · progress · layer slider                          │
└──────────────────────────────────────────────────────────┘
```

Every feature of the full app is reachable here: through menus, panels or the command palette
(`Ctrl+P` / `F3`), which runs the same commands as the console.

## Hotkeys (default, Blender-like)

| Key | Action |
|---|---|
| `Tab` | Object mode ↔ Edit mode |
| `G` / `R` / `S` | Move / Rotate / Scale, then `X` `Y` `Z` to lock axis, type a number, `Enter` |
| `1` `2` `3` (edit mode) | Vertex / Edge / Face select |
| `A` / `Alt+A` | Select all / none |
| `Shift+D` | Duplicate |
| `X` / `Delete` | Delete |
| `E` | Extrude |
| `Ctrl+Z` / `Ctrl+Shift+Z` | Undo / Redo |
| Numpad `1` `3` `7` | Front / Right / Top view |
| Middle mouse | Orbit, `Shift` pan, wheel zoom |
| `F3` / `Ctrl+P` | Command palette |
| `` ` `` | Toggle console |

The hotkey map is editable. A "Lychee-like" preset can be added later for people coming from there.
