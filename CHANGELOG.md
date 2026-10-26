# Changelog

All notable changes to Copal. Newest first.
Each version here is also published as a GitHub Release.

## 0.2.0

Supports, hollowing and drain holes: Copal can now prepare complex models for printing.

- Auto supports with Light, Medium and Heavy presets, and two modes: Auto (islands and overhangs) and Islands only
- Supports have thin tips, bodies that bend around the model, feet on the plate or on the model, cross braces and a raft
- Supports are braced to each other automatically
- Add supports one by one with a click. New supports connect to their neighbours with braces when "Connect new supports with braces" is on
- Select supports with a click (Shift+click adds), change their size, delete them with Delete
- Supports move with their model. Rotating, scaling or mirroring the model removes them
- Check islands: finds parts that would print in mid-air and marks them with red dots
- Hollow models with a wall thickness you choose, and make them solid again
- Drain holes: click on the model to add one, with your own diameter and depth. They are cut into the slices
- Export stops with a clear message when a model floats above the plate without supports
- Side panels fold away like tool windows in IntelliJ: click the — in a panel header, or press Alt+1 (tools), Alt+2 (settings), Alt+3 (console). A folded panel becomes a thin stripe; click it to bring the panel back
- The console can be hidden from the status bar
- Your own resins: fill in brand, name and color in the Resin tab and press "Save as my resin"
- Import a resin preset (.toml) with "Import file…", by dropping it on the window, or with `presets import`
- "Open folder" opens the folder with your resins. Any .toml file you put there shows up in the resin list in a few seconds, without a restart. Broken files are named in the console
- The .goo image orientation is now confirmed: Copal and Lychee give the same layers for the same model
- Console and copal-cli: `supports`, `islands`, `hollow`, `supports.*`, `holes.*`, `object.hollow`, `object.unhollow`, `presets import`, `presets folder`, `presets reload`
- Project files keep supports, drain holes and the hollow inside (older projects still open)

## 0.1.0

Copal slices! This is the first version that turns models into a printer file.

- Slicing to Elegoo .goo files: anti-aliasing, bottom and transition layers, two-stage lift and retract, wait times, previews inside the file
- Fast multithreaded slicer: overlapping and hollow models slice correctly, a 16K screen is no problem
- Import STL, OBJ and 3MF models: with the Import button, Ctrl+I, by dropping files on the window, or with `copal-cli import`
- New 3D view drawn on the graphics card, with a real perspective camera: orbit, pan, zoom to the cursor, Iso / Top / Front / Side views
- Move, rotate and scale handles (gizmos) in the 3D view. Make them bigger or smaller with the − / + buttons or in Preferences
- Drag a model with the mouse to move it over the plate: it follows the cursor and keeps its height from any camera angle
- Hold Ctrl to snap moves, rotations and scaling to steps you choose
- G / R / S switch between move, rotate and scale, like in Blender
- Duplicate (Ctrl+D), mirror X / Y / Z, drop to plate, reset rotation, arrange all models on the plate
- Models stay on the plate after rotating or scaling (can be turned off)
- Exact size fields in millimetres, with "keep proportions"
- The workflow tabs are now Layout, Prepare and Export
- Settings tabs on the right: Object, Resin, Print, Layout, Supports, Hollow. They wrap into more rows when the panel is narrow
- Save and load your own presets for exposure, motion, layout, supports and hollowing, and save your settings as your own resin
- Light, medium and heavy support presets (used by auto supports in 0.3)
- Overhang view: faces that need supports turn red (always on in Prepare)
- Layer preview in the Export step: scroll through every layer before you print
- Slicing runs in the background with a progress bar and Cancel
- Print history: every export is saved with a picture, its settings and a copy of the project exactly as it was sliced. Rate prints with stars, mark them as printed well or failed, add notes, and open the old project again
- All panels can be resized: tools, settings and the console
- A small triangle in front of the plate shows where the front is
- The Art theme now has a light version too
- Built-in file browser for opening, saving, importing and exporting (works without a desktop too)
- Status bar shows printer, resin, layer height, exposure, layer count and print time
- New printers: Elegoo Saturn 4 Ultra 12K and Elegoo Mars 5 Ultra (experimental)
- Choosing a resin applies its settings for your printer
- copal-cli: `import`, `export`, `shell` (interactive), `history`, `presets`, `printers`, `resins`, `inspect` (check any .goo file and save a layer as PNG)
- New document commands: `object.import`, `object.duplicate`, `object.mirror`, `object.arrange`, `object.remove_all`, `print.set`, `supports.set`, `hollow.set`, `layout.set`
- Console: `export`, `history`, `presets`, `settings print`, `steps` and smarter Tab completion (setting names, printers, resins)
- Project files keep all print, support, hollow and layout settings (old projects still open)

## 0.0.2

- Installers! Windows setup (.exe) and portable .exe files
- Linux packages: .deb (Ubuntu, Debian, Mint), .rpm (Fedora, openSUSE), AppImage, Flatpak bundle, PKGBUILD for Arch
- Start menu / app menu shortcuts for Copal and Copal Lite, and .copal files open in Copal
- The crystal icon is built into the Windows .exe files
- The console looks more like a terminal: the input line follows the last output, the prompt and the text line up, and a click anywhere in the console focuses it
- First presets: Elegoo Saturn 4 Ultra 16K and Anycubic ABS-Like Pro 2 Grey with tested settings
- Notes from studying Lychee Slicer: supports, hollowing, orientation and the .goo files it writes

## 0.0.1

- New default theme: calm dark or light colors with simple orange outlines
- Light mode in white and grey tones
- The amber crystal look is now the Art theme
- Settings window (Settings button, Ctrl+,) to switch theme and dark / light mode; the choice is saved
- `settings` command in the console and in copal-cli, e.g. `settings theme=art mode=light`
- The console looks and works like a terminal: `copal:project$` prompt, ↑ / ↓ history, Tab completion
- Releases are now regular releases, so the newest one shows up as "Latest" on GitHub

## 0.0.0

The foundation. Copal can't slice yet, but the engine, the apps and the release pipeline are in place.

- First code: a Rust workspace with the engine crates and three apps (Copal, Copal Lite, Copal CLI)
- Command system with undo and redo: every change is a command, the same for buttons, console and terminal
- Text command language, e.g. `object.move id=1 x=10`, with clear error messages
- `.copal` project files: save and open, safe writing, version check
- Copal CLI: `new`, `info`, `run`, `script` and `commands`, with `--json` output
- Copal and Copal Lite preview apps: amber theme with crystal borders, simple 3D view of the build plate, object inspector, scene list, built-in console, Ctrl+Z / Ctrl+Shift+Z
- Test objects (cubes) to try moving, rotating, scaling and dropping to the plate
- Automatic checks on every push (Linux and Windows) and license checks
- Automatic releases: a new version in Cargo.toml plus a CHANGELOG section is all it takes
- Issue forms for bugs, ideas, printer tests and resin results
- Project setup: logo and wordmark, README, MIT license, planning docs and the wiki
