> [!NOTE]
> The basics below work since **Copal 0.0**. Slicing, supports and the interactive shell arrive in 0.1 → 0.4.


`copal-cli` can do **everything** the apps can do. You can work with Copal using only a terminal.

## Available now (0.0)

| Command | What it does |
|---|---|
| `copal-cli new <file> [--name] [--printer] [--resin]` | Create an empty project |
| `copal-cli info <file>` | Show the project and its objects |
| `copal-cli run <file> <command> [key=value …]` | Run one command and save |
| `copal-cli script <file> <script.copalscript>` | Run a script (one command per line, `#` comments) |
| `copal-cli commands` | List every command with its arguments |

Add `--json` to any of them for machine-readable output, `--dry-run` to `run` and `script` to test
without saving.

Commands so far: `object.add_cube`, `object.move`, `object.rotate`, `object.scale`, `object.drop`,
`object.rename`, `object.remove`, `object.set_transform`, `project.rename`, `project.set_printer`,
`project.set_resin`. The console inside Copal and Copal Lite understands the same commands, plus
`undo`, `redo`, `save`, `open`, `new`, `help`.

## Coming next

## One command

```sh
copal-cli slice model.stl --printer elegoo/saturn-4-ultra-16k \
  --resin anycubic/abs-like-pro-2-grey --orient auto --supports medium -o model.goo
```

## Step by step with a project

```sh
copal-cli new part.copal --printer elegoo/saturn-4-ultra-16k
copal-cli add part.copal model.stl
copal-cli run part.copal supports.auto density=heavy
copal-cli export part.copal -o part.goo
```

## Interactive

```sh
copal-cli shell part.copal   # command prompt with autocomplete
copal-cli tui part.copal     # text interface with layer preview
```

## Scripts and macros
Save commands in a `.copalscript` file and run it with `copal-cli script file.copalscript`.
You can record a macro in the app and run it here.

## Useful flags
- `--json` — machine-readable output
- `--threads N` — limit CPU threads
- `--help` — help for any command

## Other commands
`copal-cli info file.goo` · `copal-cli convert in.ctb out.goo` · `copal-cli calibrate` ·
`copal-cli presets list`
