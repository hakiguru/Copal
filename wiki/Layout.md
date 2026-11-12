The **Layout** step is where you place models on the plate.

## Import
**Import models…** (Ctrl+I), or drop STL, OBJ or 3MF files on the window. Several files at once
work too (Ctrl+click in the file browser). New models land in a free spot on the plate.

## Move models
- **Drag a model with the mouse.** It follows the cursor over the plate and keeps its height, from
  any camera angle.
- Or use the **Move** handles (`G`): arrows for X, Y and Z, and the small square for moving over the
  plate. The up arrow lifts a model, but never pushes it into the plate.
- Or press the **arrow keys**: one snap step per press, ten with Shift. `Page Up` / `Page Down`
  lift and lower.
- Or type the position in the **Object** tab.

While you drag, a line under the 3D view shows the position, the angles or the scale.

## Rotate and scale
- **Rotate** (`R`): drag one of the three rings. `[` and `]` turn the model by one snap step.
- **Scale** (`S`): drag an end square to scale along that side of the model, or the centre square to
  scale evenly. The model grows around its middle, so it stays under the handles. `=` and `-` scale
  by one snap step. The **Size** fields in the Object tab take exact millimetres; **Keep
  proportions** scales the other sides with it.
- A model that stood on the plate stays on it after rotating or scaling (Layout tab → *Keep models
  on the plate*).

Hold **Ctrl** while dragging to snap: 1 mm, 15° and 10 % by default (Layout tab).
Too big or too small? The **− / +** buttons over the 3D view change the size of the handles.
**Move**, **Rotate** and **Scale** are over the 3D view too, next to the camera buttons.

## Other tools
The tools panel has them in rows:

| Buttons | What they do |
|---|---|
| **On plate** (`P`), **Center** (`C`) | Put the model down, or move it to the middle of the plate |
| **Auto orient** (`Ctrl+T`) | Turn the model the best way up for printing. See [[Auto orientation|Auto-Orientation]] |
| **Reset rotation** (`Ctrl+R`), **Reset all** | Back to how the model was imported |
| **Mirror X / Y / Z** (`Alt+X` / `Y` / `Z`) | Mirror the model |
| **Duplicate** (`Ctrl+D`), **Delete** (`Delete` or `X`) | Copy or remove the model |
| **Arrange all** (`Shift+A`), **Undo**, **Redo** | Pack every model on the plate with the spacing from the Layout tab |

`N` and `Shift+N` select the next and the previous model.

## Outside the plate
A model that sticks out of the build volume turns **red**, and Copal names it under the 3D view and
in the tools panel. Move, scale or arrange it: Export refuses a model that is outside the plate.

## Camera
Left-drag on empty space orbits, right or middle drag pans (or Shift+drag), the wheel zooms to the
cursor, double-click or `F` shows the whole plate. **Iso / Top / Front / Side / Fit** are over the
3D view and on the keys `4` `5` `6` `7`.

## From the console
`object.import path=model.stl`, `object.move id=1 x=10`, `object.rotate id=1 z=45`,
`object.mirror id=1 axis=x`, `object.duplicate id=1 count=3`, `object.arrange spacing=5`,
`object.center id=1`, `object.reset id=1 what=all`, `object.orient id=1`.

Short forms for all models at once: `arrange`, `drop all`, `center all`, `orient all`. In the apps the console
also moves the view and the selection: `select 2`, `select next`, `view top`, `view fit`,
`tool rotate`.
