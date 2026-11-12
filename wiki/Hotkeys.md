Press **F1** (or the **Keys** button in the top bar) to see this list inside Copal.
`hotkeys` in the console prints it with the name of every action. Letter keys work when you are
not typing in a field or in the console.

## Change any key (0.4)
Every key below can be changed. Open **Keys** (`F1`), click the key next to an action and press
the new key or combination. `Esc` cancels, `Backspace` leaves the action without a key.
**default** next to a changed key puts the old one back, **Reset all keys** resets everything.
When the same key is used twice, the window says so.

Your keys are saved with the app settings and work in Copal and Copal Lite. From the terminal:

```
copal-cli settings key.orient=Ctrl+Shift+O    # a new key
copal-cli settings key.redo="Ctrl+Shift+Z, Ctrl+Y"   # two keys
copal-cli settings key.arrange=none           # no key
copal-cli settings key.orient=default         # back to the default
```

The names (`orient`, `mode_move`, `layer_up` …) are listed by `hotkeys` in the console. Below are
the default keys.

## Project
| Key | Action |
|---|---|
| `Ctrl+O` / `Ctrl+S` / `Ctrl+Shift+S` | Open / Save / Save as |
| `Ctrl+I` | Import models |
| `Ctrl+E` | Slice & export |
| `Ctrl+Z` / `Ctrl+Shift+Z` / `Ctrl+Y` | Undo / Redo |
| `Ctrl+,` | Preferences |
| `Ctrl+Shift+P` | Print history |
| `F1` | The list of shortcuts, where you can change them |

## Steps and windows
| Key | Action |
|---|---|
| `1` / `2` / `3` | Layout / Prepare / Export |
| `Alt+1` / `Alt+2` / `Alt+3` | Show or hide tools / settings / console |
| `` ` `` | Type in the console |
| `Esc` | Cancel a running export, leave the Select / Add tool, or deselect |

## 3D view
| Key | Action |
|---|---|
| `4` / `5` / `6` / `7` | Iso / top / front / side view |
| `F` / double-click | Show the whole plate and every model |
| `O` | Overhang colours on or off |
| `N` / `Shift+N` | Select the next / previous model |
| `Alt+Up` / `Alt+Down` | Layer slider: one layer up / down (with `Shift`: ten) |
| `Alt+Home` | Layer slider: show every layer |
| Left drag | Orbit (on empty space) or move a model over the plate |
| Right / middle drag, `Shift` + drag | Pan |
| Wheel | Zoom to the cursor |

## Layout
| Key | Action |
|---|---|
| `G` / `R` / `S` | Move / Rotate / Scale mode (the handles in the 3D view) |
| Ctrl while dragging | Snap to steps |
| Arrows | Move the model by one snap step (`Shift`: ten steps) |
| `Page Up` / `Page Down` | Lift / lower by one snap step |
| `[` / `]` | Turn left / right by one snap step |
| `=` / `-` | Bigger / smaller by one snap step |
| `P` | Put on the plate |
| `C` | Center on the plate |
| `Alt+X` / `Alt+Y` / `Alt+Z` | Mirror |
| `Ctrl+R` | Reset rotation |
| `Ctrl+T` | Auto orient: turn the best way up for printing |
| `Ctrl+D` | Duplicate |
| `X` / `Delete` | Delete |
| `Shift+A` | Arrange all models |

The snap steps are in the **Layout** tab: 1 mm, 15° and 10 % by default.

## Prepare
| Key | Action |
|---|---|
| `Ctrl+P` | Prepare for printing: orientation, hollow, drain holes, supports |
| `V` / `B` / `A` / `H` | View / Select supports / Add support / Add hole |
| `Shift`+click (Select tool) | Add a support to the selection |
| `Delete` | Delete the selected supports |
| `Ctrl+U` / `Ctrl+Shift+U` | Auto supports / Remove supports |
| `Ctrl+1` / `Ctrl+2` / `Ctrl+3` | Light / Medium / Heavy supports |
| `I` | Check islands |
| `Ctrl+H` / `Ctrl+Shift+H` | Hollow / Make solid |
| `Ctrl+J` | Drain holes at the lowest points |

`Ctrl+P`, `Ctrl+U`, `Ctrl+H` and `Ctrl+J` work from any step and take you to Prepare.
`Ctrl+T` works from any step too.

## Export
| Key | Action |
|---|---|
| `L` | Layer preview on or off |
| ↑ / ↓, wheel | Next / previous layer in the layer preview |

## Console
| Key | Action |
|---|---|
| ↑ / ↓ | Earlier / later commands |
| `Tab` | Complete a command, a setting or a name |

Coming with Blender-style editing (0.6): `Tab` for Edit mode, `X` `Y` `Z` axis locks after
`G` / `R` / `S`, typed values, vertex / edge / face selection.
