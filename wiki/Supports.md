Supports hold every part of the model that would otherwise print in mid-air. Everything here is
in the **Prepare** step, in the **Supports** part of the tools panel on the left.

## Auto supports (0.2)
1. Pick a preset: **Light**, **Medium** or **Heavy**. Light uses thin tips that are easy to remove,
   Heavy uses thick, dense supports for big or heavy parts.
2. Pick a mode:
   - **Auto** — a full set: every island and every overhang steeper than the overhang angle.
   - **Islands only** — only the points that would print in mid-air. Few supports, for models that
     are mostly self-supporting.
3. Press **Auto supports**. With a model selected only that model gets supports, otherwise all of
   them. It runs in the background; a big model takes a few seconds.

What the generator does:
- lifts the model to the lift height (Supports tab) plus the raft,
- finds islands from the layers and supports each one,
- spreads supports over overhangs, denser near the plate where the peel force is highest,
- bends a support around a part of the model below it, or lets it start on the model when there
  is no way down,
- connects neighbouring supports with zigzag braces (**Auto bracing** in the Supports tab),
- puts a chamfered raft under all feet (**Raft** in the Supports tab).

Supports you placed by hand stay when you run auto supports again.

## Placing supports by hand
Choose the **Add** tool and click on the model: a support goes exactly there. With **Connect new
supports with braces** checked, it braces itself to its nearest neighbours. The first support on a
model that sits on the plate lifts the model.

## Selecting, resizing and deleting
Choose the **Select** tool and click a support (it turns orange). **Shift+click** adds more.
Then:
- change **Body** and **Tip** diameter and press **Apply size**,
- press **Delete** (or **Delete selected**) to remove them with their braces,
- **Esc** clears the selection.

**Remove supports** removes all supports of the selected model (or of all models).

## Islands check
**Check islands** looks for parts of a layer that nothing holds and marks them with red dots in the
3D view. "All islands are supported" means the model is safe to slice.

## Supports and moving the model
- Moving the model (drag, gizmo or the Position fields) keeps its supports.
- Rotating, scaling or mirroring removes them, because they no longer fit. Undo brings them back.
- **Export** refuses a model that floats above the plate without supports.

## Settings (Supports tab)
| Setting | What it does |
|---|---|
| Tip diameter, tip length | the contact point on the model |
| Diameter | the body of a support |
| Base diameter | the foot on the raft |
| Overhang angle | faces steeper than this get supports (also the red overhang colour) |
| Density | how close supports stand |
| Safe distance | how far support bodies keep from the model |
| Lift height | gap between the plate and the model |
| Auto bracing, bracing spacing | braces between supports |
| Manual bracing | braces for supports you add by hand |
| Raft, raft thickness, raft margin | the plate under the supports |

Save your own mix as a preset with the **Supports presets** row.

## From the console or terminal
```
supports auto all          # auto supports for every model
supports islands 1         # islands only, model 1
supports list              # how many supports each model has
islands all                # check islands
supports clear 1
supports.add id=1 x=10 y=-4 z=22     # one support at a point on the model
supports.remove id=1 sid=4,7
supports.resize id=1 sid=4 diameter=1.2 tip=0.5
```
`copal-cli run part.copal supports.auto id=1` does the same without opening the app.

## Planned
Tree supports, support painting, proximity and suction-cup checks, STL export with supports.
