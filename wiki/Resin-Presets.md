## Built-in presets
Starting points with a source for every value. Today: Anycubic ABS-Like Resin Pro 2 Grey on the
Elegoo Saturn 4 Ultra 16K (settings from real prints). Choosing a resin applies its settings for
your printer.

## Your own presets (0.1)
Every settings tab has a **Presets** row: **Load…**, a name field with **Save**, and **Delete…**.

| Kind | Tab | What it keeps |
|---|---|---|
| `exposure` | Resin | layer height, exposure, bottom layers and exposure, transition layers, light power |
| `motion` | Print | anti-aliasing, lift and retract (two stages), waits |
| `layout` | Layout | keep on plate, spacing, snapping |
| `supports` | Supports | every support setting (built in: light, medium, heavy) |
| `hollow` | Hollow | wall thickness, precision, drain holes |
| `resin` | Resin → My resin | all print settings as a resin for this printer; it appears in the resin list |

From the terminal: `copal-cli presets save part.copal exposure "grey 21C"`,
`copal-cli presets load part.copal supports heavy`, or `presets …` in the console.

## Why not just one database?
The same resin prints differently in another color, another batch, or a colder room.
That is why Copal puts calibration first (0.2) and presets second.

## Where they live
`~/.config/copal/presets` (Linux), `%APPDATA%\Copal\presets` (Windows). Plain TOML files —
you can back them up or share them. Updates never overwrite them.
