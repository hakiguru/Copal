`copal-cli` can do everything the apps can do. You can work with Copal using only a terminal.

## Quick start

```sh
copal-cli new part.copal --printer elegoo/saturn-4-ultra-16k --resin anycubic/abs-like-pro-2-grey
copal-cli import part.copal model.stl other.obj --arrange
copal-cli run part.copal object.prepare id=1 hollow=true   # turn, hollow, drain holes, supports
copal-cli render part.copal -o part.png                    # a picture of the result
copal-cli run part.copal print.set exposure_s=1.4 bottom_exposure_s=29
copal-cli export part.copal -o part.goo
```

That is a whole print from a model file, without opening a window.

`export` prints the layer count, resin volume and print time, and saves the export to the
[[print history|Print-History]].

## Commands

| Command | What it does |
|---|---|
| `new <file> [--name] [--printer] [--resin]` | Create a project |
| `info <file> [--settings]` | The project, its objects and (optionally) every setting |
| `import <file> <models…> [--arrange]` | Add STL / OBJ / 3MF models |
| `run <file> <command> [key=value …]` | Run one document command and save |
| `script <file> <script.copalscript>` | Run a script (one command per line, `#` comments) |
| `export <file> [-o out.goo] [--no-history]` | Slice and write the printer file |
| `shell [file]` | Interactive shell, the same as the console in the apps |
| `history [show|rate|open|remove|folder] …` | The print history |
| `presets [save|load <project> <kind> <name>] [delete <kind> <name>] [import <file.toml>] [folder]` | Your presets, printers and resins |
| `printers`, `resins [--printer id] [--all]` | What Copal knows |
| `inspect <file.goo> [--layer N --png out.png]` | Check a printer file, save a layer as a picture |
| `render <file> [-o out.png] [--view iso] [--size 1200]` | A picture of the plate: models grey, supports orange. Views: `iso`, `front`, `back`, `left`, `right`, `top`, `bottom`, `under` |
| `commands` | Every document command with its arguments |
| `settings [theme=…] [mode=…] [gizmo=…]` | The app look |

Add `--json` for machine-readable output, `--dry-run` to `run` and `script` to test without saving.

## Document commands

`object.import`, `object.add_cube`, `object.move`, `object.rotate`, `object.scale`,
`object.set_transform`, `object.mirror`, `object.duplicate`, `object.drop`, `object.center`,
`object.reset`, `object.orient`, `object.prepare`, `object.arrange`, `object.rename`,
`object.remove`, `object.remove_all`, `object.hollow`, `object.unhollow`, `supports.auto`, `supports.add`, `supports.remove`,
`supports.resize`, `supports.clear`, `holes.add`, `holes.auto`, `holes.remove`, `holes.clear`,
`project.rename`, `project.set_printer`, `project.set_resin`, `print.set`, `supports.set`,
`hollow.set`, `layout.set`.

`copal-cli commands` prints them all with their arguments.

`help print.set` in the shell lists every setting of a section; `settings print` shows the values.

## The shell

`copal-cli shell part.copal` gives a `copal:part$` prompt with `help`, `undo`, `redo`, `steps`,
`save`, `open`, `new`, `import`, `export`, `info`, `settings`, `printers`, `resins`, `presets`,
`history` and every document command. `exit` quits.

Short commands that work on one model (`2`) or on all of them (`all`, or nothing):

| Command | What it does |
|---|---|
| `prepare [id\|all] [hollow\|solid] [keep] [goal] [islands\|no-supports]` | Ready to print in one step: orientation, hollow, drain holes, supports, check. `keep` leaves the orientation |
| `orient [id\|all] [balanced\|strong\|supports\|fast]` | Turn models the best way up |
| `supports [auto\|islands\|clear\|list] [id\|all]` | Add or remove supports, or count them. `supports 1` is short for `supports auto 1` |
| `overhangs [id\|all]` | How much hangs over: area, lowest points, edges |
| `islands [id\|all]` | Islands that no support holds |
| `hollow [id\|all]`, `unhollow [id\|all]` | Hollow with the wall from the settings, or make solid |
| `holes [auto\|clear\|check] [id\|all] [count]` | Drain holes at the lowest points; `check` says if they open every hollow; alone it lists them |
| `arrange [spacing]`, `drop [id\|all]`, `center [id\|all]` | Place models on the plate |
| `settings [print\|supports\|hollow\|layout]` | Show project settings |
| `settings theme=art mode=light gizmo=1.2` | Change the app look (the same file the apps use) |

## Prepare, supports, hollowing, printers and resins
```
copal-cli run part.copal object.prepare id=1 hollow=true   # everything in one step
copal-cli run part.copal object.prepare id=1 orient=off supports=islands
copal-cli run part.copal object.orient id=1 goal=strong   # only turn it: balanced, strong, supports, fast
copal-cli run part.copal supports.auto id=1            # auto supports (mode=islands for islands only)
copal-cli run part.copal object.hollow id=1 thickness=2.5
copal-cli run part.copal holes.auto id=1               # drain holes at the lowest points
copal-cli run part.copal holes.add id=1 x=-14 y=-35 z=30 diameter=3   # as deep as the wall
copal-cli presets save part.copal printer "Garage Saturn"   # the project's printer as your own template
copal-cli presets import "My Printer.toml" "My Resin.toml"  # add printer and resin files
copal-cli presets folder                               # where your printers and resins live
```
See [[Prepare]], [[Auto orientation|Auto-Orientation]], [[Supports]],
[[Hollowing and drain holes|Hollowing-and-Drain-Holes]] and
[[Add a printer or resin|Adding-Printers-and-Resins]].

## Coming later
`copal-cli calibrate` (0.3), `convert` between formats (0.8), a text UI with a layer preview (0.8).
