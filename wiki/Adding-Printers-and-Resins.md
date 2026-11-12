Printers and resins are plain text files (`.toml`). You can add your own in a minute, and share
them with others or with the project.

## Your own templates (0.3)
Copal keeps your templates in two folders, one for printers and one for resins:

| System | Printers | Resins |
|---|---|---|
| Linux | `~/.config/copal/presets/printers/` | `~/.config/copal/presets/resins/` |
| Windows | `%APPDATA%\Copal\presets\printers\` | `%APPDATA%\Copal\presets\resins\` |

Every `.toml` file in them is one printer or one resin. Copal reads the folders while it runs: put
a file there and it shows up in the list within a few seconds, without a restart. Updates of Copal
never touch them. Each folder has a `README.txt` with the file format.
`presets folder` in the console prints both paths; **Open folder** in the Print tab (printers) and
in the Resin tab (resins) opens them.

### Add a printer
The easy way:
1. Choose a printer close to yours.
2. **Settings → Print → My printer**: type a name and press **Save as my printer**
   (console: `presets save printer <name>`).
3. Press **Open folder**, open the new file in a text editor and change the numbers: screen
   resolution, display size, build height. Save it; Copal picks up the change.

Or write the file yourself. Only three parts are required:

```toml
brand = "Elegoo"
model = "Saturn 4 Ultra 16K"

[screen]
resolution = [15120, 6230]       # pixels, X and Y
size_mm = [211.68, 118.37]       # size of the display area
mirror_y = true                  # flip the image top to bottom

[build]
height_mm = 220.0

[file]
format = "goo"
```

A printer without an `id` is `my/<file name>`. A file with the `id` of a built-in printer replaces
the built-in one. Copal writes `.goo` files today: a printer with another format can be laid out
and prepared, and exports as soon as Copal learns that format.

### Add a resin
Set the exposure and motion, then **Settings → Resin → My resin → Save as my resin**.
See [[Resin presets|Resin-Presets]] for the file format.

### Import a file someone shared
**Import file…** (Print tab or Resin tab), drop the file on the Copal window, or
`presets import <file.toml>`. Copal tells printers and resins apart by itself and puts each in its
folder. A broken file is named in the console with the reason.

**Delete** next to **Save as my printer** removes the file of the personal printer you have chosen.

## Share with the project *(0.7)*
The built-in database is open too.

### Add a printer
1. Take your printer file, or copy one from `presets/printers/` in the
   [repository](https://github.com/hakiguru/Copal).
2. Fill in the screen resolution, display size, build height and file format from the
   **manufacturer's spec page**. Put the link in `[sources]`.
3. Open a pull request, or attach the file to an issue.

### Confirm a printer works
Print a test model and open a **Printer test report** issue with a photo. Your printer becomes ✅ Stable.

### Add a resin
1. Run the [[Calibration wizard|Calibration-Wizard]].
2. Use **Share result** to export your preset.
3. Attach it to a **Resin test report** issue.

### Rules
- Every value needs a source: a link or your own test.
- Don't copy settings databases from other slicers.
