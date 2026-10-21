> [!NOTE]
> 🚧 This page describes a planned feature. It arrives in **Copal 0.1 → 0.4**. Details may change.


`copal-cli` can do **everything** the apps can do. You can work with Copal using only a terminal.

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
