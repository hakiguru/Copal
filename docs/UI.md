# Interface

## Full app — layout

Inspired by the feel of LycheeSlicer: dark, calm, everything one click away.
All icons, art and wording are our own.

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│ ◆ COPAL  [1 LAYOUT] 2 PREPARE 3 EXPORT │ Printer ▾ │ Resin ▾ │ Open Save History Preferences │
├──────────┬──────────────────────────────────────────────┬────────────────────────┤
│ tools of │ Iso Top Front Side  Overhangs  Handles − +   │ Object Resin Print     │
│ the step │                                              │ Layout Supports Hollow │ ← wraps
│          │              3D view (OpenGL)                │────────────────────────│
│ Import   │              gizmo on the selection          │ settings of the tab    │
│ Move …   │                     ▲ front                  │ + presets row          │
│ Objects  │                                              │                        │
├──────────┴──────────────────────────────────────────────┴────────────────────────┤
│ copal:project$ console                                                           │
├──────────────────────────────────────────────────────────────────────────────────┤
│ status: printer · resin · layer height · exposure · layers ≈ time · file         │
└──────────────────────────────────────────────────────────────────────────────────┘
```

- **Steps** in the top bar: Layout → Prepare → Export. The tool shelf on the left changes with the step.
  - **Layout:** import, add a test cube, move / rotate / scale (gizmo), mirror, drop to plate, reset
    rotation, duplicate, delete, arrange, undo / redo, object list.
  - **Prepare:** overhang check (red faces), supports and hollowing (0.2 / 0.3), object list.
  - **Export:** estimates, Slice & export (background, progress, cancel), layer preview, recent prints.
- **Settings tabs** on the right: Object (position, rotation, scale, exact size), Resin (layers and
  exposure, my resin), Print (anti-aliasing, two-stage lift / retract, waits, printer info), Layout
  (keep on plate, spacing, snapping, gizmo size), Supports, Hollow. Tabs wrap into more rows when the
  panel is narrow. Every tab except Object has a presets row: load, save, delete.
- **All panels are resizable:** drag the edge of the tools, settings and console panels.
- **Panels fold away** like IntelliJ tool windows: the — button in a header hides a panel, a folded
  side panel becomes a 26 px stripe with its name (click to open), the status bar has a Console
  switch, Alt+1 / Alt+2 / Alt+3 toggle tools / settings / console. The state is saved in the app
  settings (`panels`). Lite has the same switches in its View menu.
- **Prepare tools:** support presets (Light / Medium / Heavy), mode (Auto / Islands only), Auto
  supports, the View / Select / Add tools, manual bracing checkbox, size and delete for selected
  supports, Check islands, Remove supports, Hollow / Make solid with wall thickness, Add holes with
  diameter and depth. In Prepare the selected model gets only a light tint, so its supports stay
  visible.
- **3D view:** left-drag on empty space orbits, right / middle drag or Shift+drag pans, the wheel zooms
  to the cursor, double-click frames the plate. Left-drag on a model moves it over the plate; it
  keeps its height from any camera angle. A small triangle marks the front of the plate.
- **Gizmos:** move arrows plus a square for moving on the plate, rotate rings, scale handles on the
  model's own axes plus a centre square for even scaling. Ctrl snaps to the steps in the Layout tab.
  The − / + buttons over the view (and Preferences) change the handle size.
- **Windows:** file browser (works without a desktop), print history, preferences.

## Copal Lite — layout

Like mslicer: one window, plain panels, no animations. Same workspace and features as the full app.

```
┌──────────────────────────────────────────────────────────┐
│ File  Edit  Help    Layout  Prepare  Export               │
├────────────────┬─────────────────────────┬───────────────┤
│ Printer ▾      │                         │ settings tabs │
│ Resin   ▾      │       3D view           │               │
│ tools of step  │                         │               │
├────────────────┴─────────────────────────┴───────────────┤
│ console                                                   │
├──────────────────────────────────────────────────────────┤
│ status                                                    │
└──────────────────────────────────────────────────────────┘
```

## Themes

| | Default (dark / light) | Art |
|---|---|---|
| Colors | Neutral greys or white, no brown | Warm browns |
| Outlines | Simple 1 px orange line, small rounding | Amber crystal material, cut corners, glints |
| Console | Plain terminal: `copal:project$` prompt, green / blue / white | Amber log with `›` prompt |
| Light mode | Yes | Yes (parchment and amber) |

Settings live in `settings.json` in the user config folder and are shared by Copal, Copal Lite and
copal-cli. Switch in the Settings window (Ctrl+,), with `settings theme=… mode=…` in the console, or
`copal-cli settings …`.

## Console

- Prompt like bash: `copal:<project>$`.
- ↑ / ↓ go through typed commands. Tab completes command names, setting names (`print.set expo` →
  `exposure_s=`), printers, resins, history ids; when several match it lists them.
- The console runs the same shell as `copal-cli shell`: `help`, `commands`, `undo`, `redo`, `steps`,
  `new`, `open`, `save`, `import`, `export`, `info`, `settings [section]`, `printers`, `resins`,
  `presets …`, `history …`, plus every document command. App-only: `clear`, `preferences`.

## Hotkeys

| Key | Action |
|---|---|
| `G` / `R` / `S` | Move / Rotate / Scale gizmo |
| Ctrl while dragging | Snap to steps |
| `Ctrl+D` | Duplicate |
| `X` / `Delete` | Delete |
| `Esc` | Deselect, leave the Select / Add tool, or cancel a running export |
| `Delete` (Prepare) | Delete the selected supports |
| `Alt+1` / `Alt+2` / `Alt+3` | Show or hide tools / settings / console |
| `F` / double-click | Frame the plate and models |
| `Ctrl+Z` / `Ctrl+Shift+Z` / `Ctrl+Y` | Undo / Redo |
| `Ctrl+O` / `Ctrl+S` / `Ctrl+Shift+S` | Open / Save / Save as |
| `Ctrl+I` | Import models |
| `Ctrl+E` | Slice & export |
| `Ctrl+,` | Preferences |
| Left drag | Orbit (empty space) or move a model over the plate |
| Right / middle drag, Shift+drag | Pan |
| Wheel | Zoom to the cursor |

Planned (0.6, Blender-style editing): Tab for Edit mode, axis locks after G / R / S, typed values,
vertex / edge / face selection, extrude. The hotkey map becomes editable in 0.4.
