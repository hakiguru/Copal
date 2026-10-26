Hollow models use less resin, weigh less and crack less often. Everything here is in the
**Prepare** step, in the tools panel on the left.

## Hollowing (0.2)
1. Set the **Wall** thickness (2–3 mm for most figures).
2. Press **Hollow**. With a model selected only that model is hollowed, otherwise all of them.

Copal builds an inner wall that follows the outside at that distance and leaves the inside empty.
The model in the 3D view does not change; the slices show the hollow (Export → layer preview).
**Make solid** undoes the hollowing of the selected model. Precision is in the **Hollow** tab
(voxel size: smaller is more exact and slower).

## Drain holes
Every hollow part needs holes, or resin stays trapped inside and the part can crack.
Use at least two: one lets resin out, one lets air in.

1. Set the hole **Ø** (diameter) and **depth**. The depth must be more than the wall.
2. Choose **Add holes** and click on the model. Put holes low (near the plate) and in hidden spots.
3. **Remove holes** removes all holes of the selected model.

Holes are cut through the wall in the slices. They move and turn with the model.

## From the console or terminal
```
hollow all                 # hollow every model with the wall from the Hollow tab
object.hollow id=1 thickness=2.5
object.unhollow id=1
holes.add id=1 x=-14 y=-35 z=30 diameter=3 depth=4
holes.clear id=1
```

## Planned
Resin trap check, hole suggestions, infill, hollow blockers.
