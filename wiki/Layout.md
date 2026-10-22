The **Layout** step is where you place models on the plate.

## Import
**Import models…** (Ctrl+I), or drop STL, OBJ or 3MF files on the window. Several files at once
work too (Ctrl+click in the file browser). New models land in a free spot on the plate.

## Move models
- **Drag a model with the mouse.** It follows the cursor over the plate and keeps its height, from
  any camera angle.
- Or use the **Move** handles (`G`): arrows for X, Y and Z, and the small square for moving over the
  plate.
- Or type the position in the **Object** tab.

## Rotate and scale
- **Rotate** (`R`): drag one of the three rings.
- **Scale** (`S`): drag an end square to scale along that side of the model, or the centre square to
  scale evenly. The **Size** fields in the Object tab take exact millimetres; **Keep proportions**
  scales the other sides with it.
- A model that stood on the plate stays on it after rotating or scaling (Layout tab → *Keep models
  on the plate*).

Hold **Ctrl** while dragging to snap: 1 mm, 15° and 10 % by default (Layout tab).
Too big or too small? The **− / +** buttons over the 3D view change the size of the handles.

## Other tools
**Mirror X / Y / Z**, **Drop to plate**, **Reset rotation**, **Duplicate** (Ctrl+D), **Delete**
(Delete or X), **Arrange all** (packs every model on the plate with the spacing from the Layout tab).

## Camera
Left-drag on empty space orbits, right or middle drag pans (or Shift+drag), the wheel zooms to the
cursor, double-click or `F` frames the plate.

## From the console
`object.import path=model.stl`, `object.move id=1 x=10`, `object.rotate id=1 z=45`,
`object.mirror id=1 axis=x`, `object.duplicate id=1 count=3`, `object.arrange spacing=5`.
