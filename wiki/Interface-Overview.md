> [!NOTE]
> 🚧 This page describes a planned feature. It arrives in **Copal 0.4**. Details may change.


## Copal

- **Top bar** — menus, printer and resin pickers.
- **Workflow tabs** (left) — **Prepare → Supports → Slice & Export**. The tool shelf under them
  changes with the step.
- **3D viewport** (center) — the build plate and your models.
- **Inspector** (right) — settings of the selected object or tool.
- **Scene list** (right, bottom) — objects, supports, rafts, modifiers.
- **Bottom bar** — the console and the layer slider.

## Copal Lite

One window with a menu bar, a left panel (printer, resin, objects, transform, supports, slice)
and the 3D view. Every feature is also in the **command palette** (`F3` or `Ctrl+P`).

## Themes

Open **Settings** (button in the top bar, or Ctrl+,):

- **Default** — calm colors with simple orange outlines, in **dark** or **light** mode.
- **Art** — warm amber with crystal borders, like the logo (dark only).

The choice is saved and shared by Copal, Copal Lite and copal-cli.
From the console: `settings theme=art`, `settings theme=default mode=light`.

## Console

It works like a terminal: a `copal:project$` prompt, ↑ / ↓ for earlier commands, Tab to complete.
Type the same commands as in [[Command line|Command-Line]].
Everything you click is also shown there as a command, so you can learn and copy it.
