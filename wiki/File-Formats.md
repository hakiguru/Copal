> [!NOTE]
> 🚧 This page describes a planned feature. It arrives in **Copal 0.1 → 0.7**. Details may change.


Each printer brand uses its own file format. Copal writes and reads:

| Format | Printers | Version |
|---|---|---|
| `.goo` | Elegoo (newer models) | 0.1 |
| `.ctb` | Chitu boards: older Elegoo, Phrozen, many others | 0.1 |
| Photon Workshop (`.pwmx`, `.pwma`, `.pm3`, `.pm4n`, `.pm5`, `.pm7` …) | Anycubic | 0.7 |
| `.cxdlp` | Creality Halot | 0.7 |
| `.prz` | Phrozen (newer models) | 0.7 |
| `.sl1`, `.sl1s` | Prusa | 0.7 |
| `.jxs` | Uniformation GKone | 0.7 |
| NanoDLP / Klipper / generic `.zip` | many | 0.7 |
| `.lgs`, `.fdg`, `.cws`, `.zcode`, `.svgx`, `.osf`, LUMEN … | others | later |

**Which one does my printer use?** Select your printer in Copal and it picks the format for you.
