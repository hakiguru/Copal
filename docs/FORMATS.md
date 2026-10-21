# Printer file formats

Resin printers don't share one file format. Each brand (often each board maker) has its own.
Ambra needs a **writer** for every format to print, and a **reader** to open sliced files
for inspection and conversion.

## Priorities

### Tier 1 — first release (0.1)

| Format | Used by | Notes |
|---|---|---|
| `.goo` | Elegoo (Saturn 3/4 Ultra, Mars 4/5 Ultra and newer) | **Our main test printer.** |
| `.ctb` | Chitu boards: older Elegoo, Phrozen, Uniformation, many others | Several versions; newer ones are encrypted. |

### Tier 2 — big database release (0.7)

| Format | Used by |
|---|---|
| Photon Workshop family: `.pwmx`, `.pwma`, `.pwmb`, `.pwms`, `.pmx2`, `.pm3`, `.pm3m`, `.pm3r`, `.pm4n`/`.pm4u`, `.pm5`, `.pm5s`, `.pm7`, `.pm7m` and others | Anycubic Photon Mono series |
| `.cxdlp`, `.cxdlpv4` | Creality Halot series |
| `.prz` | Phrozen (newer models) |
| `.sl1`, `.sl1s` | Prusa SL1 / SL1S |
| `.jxs` | Uniformation GKone |
| NanoDLP `.zip` | NanoDLP-based printers |
| Klipper `.zip` | Klipper-based resin printers |
| Generic / Chitubox `.zip` | Many printers with image + G-code zips |
| PNG / SVG stack | Anything, debugging |

### Tier 3 — later

`.lgs` (Longer), `.fdg` (Voxelab), `.cws` (Wanhao, NovaMaker), `.zcode`/`.zcodex` (Uniz, Zortrax),
`.svgx` (Flashforge), `.osf`/`.vlr` (Vlare, Peopoly), `.cbddlp`/`.photon` (old Chitu),
**LUMEN** (new open format from the Open Resin Alliance, MIT spec).

> The exact list of extensions per printer model must be checked against each manufacturer's
> current slicer before release. Brands add new formats with almost every printer generation.

## Rules

- **Clean-room.** We write our own encoders from public format docs and from inspecting files
  we sliced ourselves. We do not copy code from GPL/AGPL tools (UVtools, mslicer, PrusaSlicer).
- **Golden tests.** For each format: a tiny model, a committed sample file, and a test that
  our writer produces a file the reader understands byte-for-byte where possible.
- **External check.** In CI we may open our files with third-party tools (e.g. UVtools'
  command line) just to confirm they are valid. Using a tool is fine; copying its code is not.
- **Real prints.** A format is *stable* only after a real print on real hardware.
  Until then it is marked *experimental* in the UI.
- Writers stream layers to disk. Never keep all layers of a 16K print in memory.

## Network sending

| Protocol | Printers | When |
|---|---|---|
| SDCP (Chitu / CBD, over Wi-Fi) | Elegoo Saturn 4 Ultra and other recent Chitu-board printers | 0.8 |
| NanoDLP HTTP API | NanoDLP printers | 0.8 |
| Others (Anycubic, Creality) | depends on public docs | later |
