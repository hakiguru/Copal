Every export is remembered — with the project exactly as it was sliced.

Open it with **History** in the top bar (File → Print history in Lite) or from the Export step.

For each export you see:
- a picture of the plate, the file name and date, printer and resin;
- layers, height, resin volume, print time, exposure and bottom exposure;
- **stars** (1–5), **Printed well** / **Failed**, and a **note** — what went right or wrong.

Buttons:
- **Open project** — opens the project as it was at slicing time, as a new unsaved copy. Change
  it and export again without losing the old version.
- **Show folder** — opens the folder with `record.json`, `project.copal` and `preview.png`.
- **Remove** — deletes this entry (the printer file itself stays where you saved it).

The history lives in `~/.local/share/copal/history` (Linux) or `%APPDATA%\Copal\history` (Windows).

From the terminal:

```sh
copal-cli history                       # list
copal-cli history show <id>
copal-cli history rate <id> 5 good "perfect, no layer lines"
copal-cli history open <id> again.copal # copy the snapshot to a new project
copal-cli history remove <id>
```
