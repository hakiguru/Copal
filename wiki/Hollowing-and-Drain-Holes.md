Hollow models use less resin, weigh less and crack less often. A big hollow part also pulls less
on the film than a solid one, because each layer is only a wall. Everything here is in the
**Prepare** step, in **2 · Hollow** and **3 · Drain holes** of the tools panel on the left.
**Prepare for printing** at the top of that panel does both for you when **Hollow** is ticked
(see [[Prepare]]).

## Hollowing
1. Set the **Wall** thickness (2–3 mm for most figures).
2. Press **Hollow** (`Ctrl+H`). With a model selected only that model is hollowed, otherwise all
   of them.

Copal builds an inner wall that follows the outside at that distance and leaves the inside empty.
The model in the 3D view does not change; the slices show the hollow (Export → layer preview).
Models made of several parts that run into each other (a body with a handle pushed into it) are
hollowed as one solid.
**Make solid** (`Ctrl+Shift+H`) undoes the hollowing.

The console says what was done: how much resin is saved, how many separate hollows there are and
how many pockets were filled.

### Pockets (0.3)
A thick ear or a fist can get a small hollow of its own, closed on all sides. No drain hole reaches
it, so it only traps resin. Copal fills such pockets again. **Hollow** tab → **Fill pockets under**
sets the size (50 mm³ by default, 0 keeps every hollow).

### Precision
**Hollow** tab → **Precision** is the voxel size: smaller is more exact and slower. A model too big
for the voxel size you chose gets bigger voxels by itself, and the console says which size was used.

## Drain holes
Every hollow needs holes. Without them resin stays trapped inside, and while printing the hollow
works like a suction cup on the film: that is how hollow prints come off the plate.
Use at least two per hollow: one lets resin out, one lets air in.

### Auto holes
Press **Auto holes** (`Ctrl+J`). Every separate hollow gets two holes at its lowest points, the
side that prints first, straight down through the floor and well apart. A model that stands on the
plate gets them through the side wall instead, because the plate would close a hole in the floor.

Turn the model first, then add the holes: the lowest points change with the orientation.
**Prepare for printing** places them again when it turns a model.

### By click
1. Set the hole **Ø** (diameter).
2. Choose **Add by click** (`H`) and click on the model. The hole goes in along the surface and is
   just deep enough to open the hollow, however thick the wall is there. Put holes low (near the
   plate) and in hidden spots.

On a solid model a hole gets the depth from the **Hollow** tab.
**Remove** removes the holes of the selected model, or of all models when none is selected.
Holes are cut through the wall in the slices. They move and turn with the model.

### Do the holes work? (0.3)
The line under the buttons tells you, and turns red when something is wrong:

| It says | What it means |
|---|---|
| 2 holes open the hollow | all is well |
| the hollow has no drain hole | resin stays inside: add holes |
| 1 of 2 hollows have no drain hole | a separate hollow was missed: add a hole there |
| 1 holes end in the wall | a hole does not reach the hollow: remove it and click again |
| add a second hole | one hole drains slowly, because no air gets in |

`holes check` in the console says the same. Supports keep out of the holes and hold the rim
around a hole at the lowest point of a model.

## From the console or terminal
```
hollow all                 # hollow every model with the wall from the Hollow tab
unhollow 1                 # make model 1 solid again
holes auto all             # two drain holes per hollow model, at the lowest points
holes auto 1 3             # three holes for model 1
holes                      # which models are hollow and what holes they have
holes check                # do the holes open every hollow?
holes clear 1
object.hollow id=1 thickness=2.5
holes.auto id=1 count=2 diameter=3
holes.add id=1 x=-14 y=-35 z=30 diameter=3           # as deep as the wall
holes.add id=1 x=-14 y=-35 z=30 diameter=3 depth=4   # your own depth
hollow.set min_pocket_mm3=80
```

## Planned
Infill, hollow blockers, caps for holes.
