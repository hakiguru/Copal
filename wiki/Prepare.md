The **Prepare** step makes a model ready to print. Its tools panel on the left reads from top to
bottom in the order you work:

| Part | What it is for |
|---|---|
| **Ready in one click** | everything below, done for you |
| **1 · Orientation** | turn the model the best way up |
| **2 · Hollow** | take out the inside to save resin |
| **3 · Drain holes** | let resin out of the hollow and air in |
| **4 · Supports** | hold every part that would print in mid-air |

Buttons work on the selected model, or on all models when none is selected.

## Ready in one click (0.3)
Press **Prepare for printing** (`Ctrl+P`). Copal then:

1. turns the model the best way up ([[Auto orientation|Auto-Orientation]]),
2. hollows it and adds two drain holes to every hollow, if **Hollow** is ticked
   ([[Hollowing and drain holes|Hollowing-and-Drain-Holes]]),
3. lifts it and adds supports, braces and a raft ([[Supports]]),
4. checks the result: every island must be held and every hollow must have a hole.

The console lists what was done, and names anything that still needs your eye in red.
It is one step for undo: `Ctrl+Z` takes all of it back.

The three switches under the button choose the steps:

- **Turn** — off keeps the orientation you set yourself.
- **Hollow** — on for big, bulky parts. Off by default, because small parts print fine solid.
- **Supports** — off when you want to place them yourself.

The choices in the parts below (orientation goal, wall thickness, hole size, support preset) are
the ones the button uses. After it, slice and export: **3 Export**.

## Step by step
Each part of the panel does one step on its own, with more choices:

- [[Auto orientation|Auto-Orientation]] — four goals: Balanced, Strong, Clean, Low.
- [[Hollowing and drain holes|Hollowing-and-Drain-Holes]] — wall thickness, automatic holes, holes
  by click, and a line that says if the holes really open the hollow.
- [[Supports]] — Light / Medium / Heavy, the base on the plate (raft, pads, grid or none),
  everywhere or islands only, supports by click that join the ones already there, selecting and
  deleting, the islands check.

## Overhang check
In the **Prepare** step faces that point down more steeply than the overhang angle turn red: they
will need supports. The angle is in the **Supports** tab (45° by default). In other steps the
**Overhangs** button over the 3D view (or `O`) turns the colouring on. Faces that lie on the plate
stay as they are: the plate holds them.

The **Overhangs** button in the tools panel prints the numbers: area, lowest points and edges.

## From the console or terminal
```
prepare                    # every model: turn, drain holes if hollow, supports, check
prepare hollow             # the same, and hollow the models first
prepare 1 keep hollow      # model 1, keep its orientation
prepare strong hollow      # aim for the smallest layers
prepare solid islands      # no hollow, supports for islands only
object.prepare id=1 orient=balanced hollow=true holes=true supports=auto
```
`copal-cli run part.copal object.prepare id=1 hollow=true` does the same without opening the app,
and `copal-cli render part.copal -o part.png` saves a picture of the result.

## Planned
Several build plates in one project. Measuring distances, angles and wall thickness. Repair of
holes, flipped normals and broken geometry.
