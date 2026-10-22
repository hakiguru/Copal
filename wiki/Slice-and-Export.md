The **Export** step turns your plate into a printer file.

## Before slicing
The left panel shows the printer, file type, layer count, height and an estimated print time.
Settings are in the **Resin** (layers, exposure) and **Print** (anti-aliasing, lift, retract,
waits) tabs.

## Layer preview
Press **Layers** over the 3D view (or **Show layers**). Drag the slider, use ↑ / ↓ or the mouse
wheel. The top line shows the layer number, its height and its exposure.

## Slice & export
**Slice & export…** (Ctrl+E) asks where to save, then slices in the background with a progress bar.
**Cancel** or `Esc` stops it. When it is done, the console and the left panel show the layer count,
resin volume and print time, and the export is added to the [[print history|Print-History]].

What Copal writes into the file:
- **Anti-aliasing** (2×, 4×, 8×) with real grey edge pixels.
- **Bottom layers** and **transition layers** that step the exposure down evenly.
- **Two-stage lift and retract** and the waits after exposure and after retract.
- Two preview pictures that the printer shows.

## Formats
`.goo` for Elegoo printers now; `.ctb` in 0.2, more in 0.7. See [[File formats|File-Formats]].

> [!WARNING]
> Copal 0.1 has not made a real print yet. Until the image orientation is confirmed
> (see the [roadmap](https://github.com/hakiguru/Copal/blob/main/docs/ROADMAP.md)), a model could
> come out mirrored. Try a small, non-symmetric test part first.

## Later
XY compensation (0.5), GPU slicing (0.8), sending over Wi-Fi (0.8), opening and converting sliced
files (0.8). `copal-cli inspect` already reads `.goo` files and saves any layer as a PNG.
