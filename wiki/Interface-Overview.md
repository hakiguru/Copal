## Copal

- **Top bar** — the steps **Layout → Prepare → Export** (keys `1` `2` `3`), printer and resin
  pickers, Open, Save, History, Preferences and **Keys** (the list of shortcuts, `F1`, where you
  can change any of them).
- **Tools** (left) — change with the step. Buttons of one kind stand in a row and have the same
  size. Layout: import, open from the print history, move / rotate / scale, on plate, center, reset,
  mirror, duplicate, delete, arrange, auto orient. Prepare: **Prepare for printing** at the top does everything in one
  click; below it the same steps in the order you work: 1 Orientation, 2 Hollow, 3 Drain holes,
  4 Supports (with the base on the plate: raft, pads, grid or none). Export: slice, layer
  preview, recent prints.
- **3D view** (center) — the build plate, your models and the handles of the selected model.
  Buttons on top: Iso / Top / Front / Side / Fit, Overhangs, Move / Rotate / Scale and handle size
  − / + in Layout, Layers in the Export step. On the right, the **layer slider** cuts the view at
  any layer (`Alt+Up` / `Alt+Down`). Faces that need supports are tinted soft lilac. A small
  triangle in front of the plate marks the front. A line under the view shows what a drag is doing, and warns about models outside the
  build volume (they turn red).
- **Settings tabs** (right) — **Object**, **Resin**, **Print**, **Layout**, **Supports**, **Hollow**.
  When the panel is narrow, the tabs wrap into two or more rows. Each tab except Object can save
  and load your own presets.
- **Console** (bottom) and the **status bar** with printer, resin, layer height, exposure, layer
  count and print time.

Every panel can be resized: drag its edge.

### Hiding panels
Tools, settings and the console fold away like tool windows in IntelliJ:
- the **—** button in a panel's header hides it; a folded side panel becomes a thin stripe with its
  name, click the stripe to bring it back,
- **Console** at the left of the status bar shows or hides the console,
- **Alt+1** tools, **Alt+2** settings, **Alt+3** console.

Copal remembers which panels are open. In Copal Lite the same switches are in the **View** menu.

## Copal Lite

The same features in a plain window: menus (File, Edit, View, Prepare, Help) with the steps next
to them, tools on the left, settings tabs on the right, the console at the bottom.

## Name and icon in the taskbar

The window is called **Copal Slicer** with the version (or **Copal Slicer Lite**) and has the
Copal logo in the taskbar and the dock. The packages install a desktop entry for that. The
AppImage and the `.tar.gz` write their own entry into `~/.local/share/applications` on the first
start (and update the path when you move the AppImage). If the dock still shows a gear, close the
app and start it once more.

## Themes

Open **Preferences** (top bar, or Ctrl+,):

- **Default** — calm colors with simple orange outlines.
- **Art** — warm amber with crystal borders, like the logo.

Both come in **dark** and **light**. The choice is saved and shared by Copal, Copal Lite and
copal-cli. From the console: `settings theme=art mode=light`; from the terminal:
`copal-cli settings theme=art mode=light`.

## Console

It works like a terminal: a `copal:project$` prompt, ↑ / ↓ for earlier commands, Tab to complete
command names, setting names, printers and resins. Type `help`. Press `` ` `` to jump into it. It
understands the same commands as `copal-cli shell` — see [[Command line|Command-Line]].

The console reaches everything the mouse does. On top of the shell commands the apps add:

| Command | What it does |
|---|---|
| `step layout\|prepare\|export` | Go to a workflow step |
| `select <id>\|next\|prev\|none` | Choose the model that tools work on |
| `view iso\|top\|front\|side\|fit` | Turn the camera |
| `view overhangs [on\|off]`, `view layers [on\|off\|<number>]` | Overhang colours, layer preview |
| `tool move\|rotate\|scale`, `tool view\|select\|add\|hole` | The handles (Layout) or the click tool (Prepare) |
| `panel tools\|settings\|console [on\|off]` | Show or hide a side window |
| `hotkeys`, `preferences`, `clear` | List the shortcuts, open Preferences, empty the console |
