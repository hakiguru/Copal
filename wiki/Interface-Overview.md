## Copal

- **Top bar** — the steps **Layout → Prepare → Export**, printer and resin pickers, Open, Save,
  History and Preferences.
- **Tools** (left) — change with the step. Layout: import, move / rotate / scale, mirror,
  duplicate, arrange. Prepare: auto supports, the View / Select / Add tools, islands check, hollowing and
  drain holes. Export: slice,
  layer preview, recent prints.
- **3D view** (center) — the build plate, your models and the handles of the selected model.
  Buttons on top: Iso / Top / Front / Side views, Overhangs, handle size − / +, and Layers in the
  Export step. A small triangle in front of the plate marks the front.
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

The same features in a plain window: menus (File, Edit, Help) with the steps next to them, tools on
the left, settings tabs on the right, the console at the bottom.

## Themes

Open **Preferences** (top bar, or Ctrl+,):

- **Default** — calm colors with simple orange outlines.
- **Art** — warm amber with crystal borders, like the logo.

Both come in **dark** and **light**. The choice is saved and shared by Copal, Copal Lite and
copal-cli. From the console: `settings` in copal-cli, e.g. `copal-cli settings theme=art mode=light`.

## Console

It works like a terminal: a `copal:project$` prompt, ↑ / ↓ for earlier commands, Tab to complete
command names, setting names, printers and resins. Type `help`. It understands the same commands as
`copal-cli shell` — see [[Command line|Command-Line]].
