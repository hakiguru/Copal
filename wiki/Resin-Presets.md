> [!NOTE]
> 🚧 This page describes a planned feature. It arrives in **Copal 0.1 → 0.7**. Details may change.


## Built-in presets
Starting points from manufacturers' published settings. Each value shows its source.

## Personal presets
Made by the [[Calibration wizard|Calibration-Wizard]] or by hand. They store printer, layer height,
exposure and notes like color, batch and room temperature. Updates never overwrite them.

## Why not just one database?
The same resin prints differently in another color, another batch, or a colder room.
That is why Copal puts calibration first and presets second.

## Where they live
`~/.config/copal/presets` (Linux), `%APPDATA%\Copal\presets` (Windows). Plain TOML files —
you can back them up or share them.
