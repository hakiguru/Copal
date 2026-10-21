> [!NOTE]
> 🚧 This page describes a planned feature. It arrives in **Copal 0.2 → 0.5**. Details may change.


## Import
STL (binary and text), OBJ, 3MF. Drag files into the window or use **File → Import**.

## Move, rotate, scale
- Gizmos in the viewport, or type exact numbers in the Inspector.
- Blender-style keys: `G` move, `R` rotate, `S` scale, then `X` `Y` `Z` to lock an axis.
- **Drop to plate**, **Center**, **Mirror**, **Duplicate**.

## Auto orient *(0.5)*
Finds an angle with fewer islands, less support and smaller layer areas.

## Arrange *(0.5)*
Packs all models on the plate, or fills the plate with copies.

## Multi-plate *(0.5)*
Keep several build plates in one project and slice them one by one.

## One-click prepare *(0.5)*
**Magic button**: orient → support → arrange for the selected model.
**Batch button**: the same for many models.

## Measure *(0.5)*
Measure distances, angles and wall thickness. **Physical size mode** shows the model at its
real size on your screen.

## Repair *(0.5)*
Fixes holes, flipped normals and broken geometry. Copal warns you when a model needs it.
