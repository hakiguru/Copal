`copal-cli` can do everything the apps can do. You can work with Copal using only a terminal.

## Quick start

```sh
copal-cli new part.copal --printer elegoo/saturn-4-ultra-16k --resin anycubic/abs-like-pro-2-grey
copal-cli import part.copal model.stl other.obj --arrange
copal-cli run part.copal object.rotate id=1 x=30
copal-cli run part.copal print.set exposure_s=1.4 bottom_exposure_s=29
copal-cli export part.copal -o part.goo
```

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
| `presets [save|load <project> <kind> <name>] [delete <kind> <name>]` | Your presets |
| `printers`, `resins [--printer id] [--all]` | What Copal knows |
| `inspect <file.goo> [--layer N --png out.png]` | Check a printer file, save a layer as a picture |
| `commands` | Every document command with its arguments |
| `settings [theme=…] [mode=…] [gizmo=…]` | The app look |

Add `--json` for machine-readable output, `--dry-run` to `run` and `script` to test without saving.

## Document commands

`object.import`, `object.add_cube`, `object.move`, `object.rotate`, `object.scale`,
`object.set_transform`, `object.mirror`, `object.duplicate`, `object.drop`, `object.arrange`,
`object.rename`, `object.remove`, `object.remove_all`, `project.rename`, `project.set_printer`,
`project.set_resin`, `print.set`, `supports.set`, `hollow.set`, `layout.set`.

`help print.set` in the shell lists every setting of a section; `settings print` shows the values.

## The shell

`copal-cli shell part.copal` gives a `copal:part$` prompt with `help`, `undo`, `redo`, `steps`,
`save`, `open`, `new`, `import`, `export`, `info`, `settings`, `printers`, `resins`, `presets`,
`history` and every document command. `exit` quits.

## Coming later
`supports.*` (0.3), hollowing commands (0.2), `copal-cli calibrate` (0.2), `convert` between
formats (0.8), a text UI with a layer preview (0.8).
