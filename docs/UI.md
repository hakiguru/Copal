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
  - **Layout:** import, add a test cube, move / rotate / scale (gizmo), on plate, center, auto
    orient, reset rotation / all, mirror, duplicate, delete, arrange, undo / redo, object list.
  - **Prepare:** one-click prepare, then orientation, hollow, drain holes, supports (see below),
    object list.
  - **Export:** estimates, Slice & export (background, progress, cancel), layer preview, recent prints.
- **Settings tabs** on the right: Object (position, rotation, scale, exact size), Resin (layers and
  exposure, my resin), Print (anti-aliasing, two-stage lift / retract, waits, printer info), Layout
  (keep on plate, spacing, snapping, gizmo size), Supports, Hollow. Tabs wrap into more rows when the
  panel is narrow. Every tab except Object has a presets row: load, save, delete.
- **Tool rows:** buttons that belong together stand in one row and share its width
  (`widgets::button_row`), so the panel reads as a grid, not as a paragraph of buttons. A row takes
  (label, hint, enabled); the hint names the hotkey. Disabled buttons stay in place.
- **All panels are resizable:** drag the edge of the tools, settings and console panels.
- **Panels fold away** like IntelliJ tool windows: the — button in a header hides a panel, a folded
  side panel becomes a 26 px stripe with its name (click to open), the status bar has a Console
  switch, Alt+1 / Alt+2 / Alt+3 toggle tools / settings / console. The state is saved in the app
  settings (`panels`). Lite has the same switches in its View menu.
- **Where would the user click first?** Every panel is laid out by that question. Someone who
  wants a model ready to print looks at the top of Prepare, so the one button that does it all
  is there, and it is the only primary (filled) button of the step. Someone who wants one thing
  finds it under its own numbered title, in the order the work is done.
- **Prepare tools**, top to bottom:
  - *Ready in one click:* **Prepare for printing** (primary), switches Turn / Hollow / Supports.
  - *1 · Orientation:* goal (Balanced / Strong / Clean / Low, `widgets::choice_row`), Auto orient.
  - *2 · Hollow:* wall thickness, Hollow / Make solid.
  - *3 · Drain holes:* Auto holes / Remove, Add by click with diameter, and a status line from
    `hole_check` (red when a hollow is closed or a hole is blind).
  - *4 · Supports:* preset (Light / Medium / Heavy, the one in use outlined), where (Everywhere /
    Islands only), Auto supports, the View / Select / Add tools, join and brace switches for
    supports added by hand, size and delete for selected supports, Check islands / Overhangs /
    Remove. The island check runs by itself after auto supports.
  In Prepare the selected model gets only a light tint, so its supports stay visible.
- **3D view:** left-drag on empty space orbits, right / middle drag or Shift+drag pans, the wheel zooms
  to the cursor, double-click frames the plate. Left-drag on a model moves it over the plate; it
  keeps its height from any camera angle. A small triangle marks the front of the plate.
  - Handles and models are picked at the point where the button went down (`press_pos`), not where
    egui noticed the drag a few pixels later.
  - A plate drag is ignored while the pointer is above the horizon or the point under it is farther
    than 1.5 plates away (`plate_point`), so a flat view can not throw a model away.
  - A line under the view shows the transform being dragged; models outside the build volume are
    tinted red and named there.
  - Over the view: Iso / Top / Front / Side / Fit, Overhangs, Move / Rotate / Scale (Layout),
    handle size, Layers (Export).
- **Gizmos:** move arrows plus a square for moving on the plate, rotate rings, scale handles on the
  model's own axes plus a centre square for even scaling. Scaling keeps the middle of the model
  (the gizmo centre) in place. Ctrl snaps to the steps in the Layout tab.
  The − / + buttons over the view (and Preferences) change the handle size.
- **Windows:** file browser (works without a desktop), print history, preferences, keyboard
  shortcuts (F1).

## Copal Lite — layout

Like mslicer: one window, plain panels, no animations. Same workspace and features as the full app.

```
┌──────────────────────────────────────────────────────────┐
│ File Edit View Prepare Help   Layout  Prepare  Export     │
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
  `presets …`, `history …`, `supports`, `islands`, `overhangs`, `hollow`, `unhollow`, `holes`,
  `arrange`, `drop`, `center`, plus every document command.
- App-only commands change what is on the screen, so the console reaches everything the mouse
  does: `step`, `select`, `view`, `tool`, `panel`, `hotkeys`, `preferences`, `clear`
  (`workspace::APP_HELP`). `settings theme=art` is handled by the shell and the app reloads its look.

## Hotkeys

Every key is an *action* with default keys in `copal-ui-common/src/hotkeys.rs` (`ACTIONS`). The
user's own keys live in the app settings (`keys`, `key.<action>=…` on the command line) and win
over the defaults. `Keymap::pressed` turns the key events of a frame into actions and
`Workspace::hotkeys` does them. The F1 window lists them and changes them (click, then press keys).
Keep `ACTIONS` and `wiki/Hotkeys.md` in step. Hints in the UI take the key from the keymap
(`Workspace::key_text`), never a hard-coded string. Plain keys are ignored while a text field or
the console has the focus; Ctrl keys and F-keys still work.

| Key | Action |
|---|---|
| `1` / `2` / `3` | Layout / Prepare / Export |
| `4` / `5` / `6` / `7` | Iso / top / front / side view |
| `F` / double-click | Frame the plate and models |
| `O` | Overhang colours |
| `N` / `Shift+N` | Next / previous model |
| `G` / `R` / `S` | Move / Rotate / Scale gizmo |
| Ctrl while dragging | Snap to steps |
| Arrows, `Page Up` / `Page Down` | Nudge the model by the move snap step (Shift: × 10) |
| `[` / `]`, `=` / `-` | Turn / scale by one snap step |
| `P` / `C` | On plate / center |
| `Alt+X` / `Alt+Y` / `Alt+Z` | Mirror |
| `Ctrl+R` | Reset rotation |
| `Ctrl+T` | Auto orient |
| `Ctrl+P` | Prepare for printing |
| `Ctrl+D` | Duplicate |
| `X` / `Delete` | Delete |
| `Shift+A` | Arrange all |
| `V` / `B` / `A` / `H` (Prepare) | View / select supports / add support / add hole |
| `Delete` (Prepare) | Delete the selected supports |
| `Ctrl+U` / `Ctrl+Shift+U` | Auto supports / remove supports |
| `Ctrl+1` / `Ctrl+2` / `Ctrl+3` (Prepare) | Light / medium / heavy supports |
| `I` (Prepare) | Check islands |
| `Ctrl+H` / `Ctrl+Shift+H` | Hollow / make solid |
| `Ctrl+J` | Auto drain holes |
| `L` (Export) | Layer preview |
| `Esc` | Cancel a running export, close the shortcut list, leave a tool, deselect |
| `Alt+1` / `Alt+2` / `Alt+3` | Show or hide tools / settings / console |
| `` ` `` | Focus the console |
| `F1` | The shortcut list |
| `Ctrl+Z` / `Ctrl+Shift+Z` / `Ctrl+Y` | Undo / Redo |
| `Ctrl+O` / `Ctrl+S` / `Ctrl+Shift+S` | Open / Save / Save as |
| `Ctrl+I` | Import models |
| `Ctrl+E` | Slice & export |
| `Ctrl+,` | Preferences |
| Left drag | Orbit (empty space) or move a model over the plate |
| Right / middle drag, Shift+drag | Pan |
| Wheel | Zoom to the cursor |

Keys are matched exactly (`Ctrl+U` does not fire for `Ctrl+Shift+U`); keys with Ctrl or Alt are
taken out of the input so text fields do not see them too.

Planned (0.6, Blender-style editing): Tab for Edit mode, axis locks after G / R / S, typed values,
vertex / edge / face selection, extrude.
