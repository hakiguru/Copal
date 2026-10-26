## Built-in presets
Starting points with a source for every value. Today: Anycubic ABS-Like Resin Pro 2 Grey on the
Elegoo Saturn 4 Ultra 16K (settings from real prints). Choosing a resin applies its settings for
your printer.

## Your own presets (0.1)
Every settings tab has a presets row (**Exposure presets**, **Supports presets**, …): **Load…**, a name
field with **Save**, and **Delete…**.

| Kind | Tab | What it keeps |
|---|---|---|
| `exposure` | Resin | layer height, exposure, bottom layers and exposure, transition layers, light power |
| `motion` | Print | anti-aliasing, lift and retract (two stages), waits |
| `layout` | Layout | keep on plate, spacing, snapping |
| `supports` | Supports | every support setting (built in: light, medium, heavy) |
| `hollow` | Hollow | wall thickness, precision, drain holes |
| `resin` | Resin → My resin | all print settings as a resin for this printer; it appears in the resin list (see below) |

From the terminal: `copal-cli presets save part.copal exposure "grey 21C"`,
`copal-cli presets load part.copal supports heavy`, or `presets …` in the console.

## Your own resins (0.2)
In **Settings → Resin → My resin**:
- fill in **Brand**, **Name** and **Color** and press **Save as my resin**. It saves everything
  from the Resin and Print tabs (exposure, waits, lift and retract) for the current printer and
  picks the new resin. Saving again with the same name updates it; a resin made for another printer
  keeps its other printers.
- **Delete** removes the file of the personal resin you have chosen.
- **Import file…** adds a resin preset (`.toml`), for example one a friend shared. Dropping a
  `.toml` file on the Copal window does the same.
- **Open folder** opens the resin folder in your file manager.

### The resin folder
Every `.toml` file in the folder is one resin. Copal checks the folder every second or two while
it runs: copy a file in and it appears in the resin list, edit it and the changes load, delete it
and it disappears. Files that can't be read are listed in red with the reason. The folder has a
`README.txt` with the format; the smallest file that works is:

```toml
brand = "Anycubic"
name = "ABS-Like Pro 2"
color = "Grey"

[[profiles]]
printer = "elegoo/saturn-4-ultra-16k"     # copal-cli printers lists the ids
exposure_s = 1.4
bottom_exposure_s = 29.0
```

Everything you leave out gets Copal's default. Add one `[[profiles]]` per printer. A file with the
`id` of a built-in resin (for example `id = "anycubic/abs-like-pro-2-grey"`) replaces it.

From the terminal:
```
copal-cli presets folder                         # where the folder is
copal-cli presets import "Siraya Fast.toml"      # copy a file in
copal-cli presets reload                         # check every file
presets save resin ABS 30um brand=Anycubic color=Grey   # in the console or copal-cli shell
```

Lychee `.lyr` profiles are encrypted, so Copal can't read them. Copy the numbers from Lychee's
resin settings into the Resin tab instead, then save them as your resin.

## Why not just one database?
The same resin prints differently in another color, another batch, or a colder room.
That is why Copal puts calibration first (0.3) and presets second.

## Where they live
`~/.config/copal/presets` (Linux), `%APPDATA%\Copal\presets` (Windows). Plain TOML files —
you can back them up or share them. Updates never overwrite them.
