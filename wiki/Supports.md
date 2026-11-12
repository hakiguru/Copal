Supports hold every part of the model that would otherwise print in mid-air. Everything here is
in the **Prepare** step, in **4 · Supports** of the tools panel on the left.
**Prepare for printing** at the top of that panel adds supports too, with the choices made here
(see [[Prepare]]).

## Auto supports
1. Pick a preset: **Light**, **Medium** or **Heavy** (`Ctrl+1` / `2` / `3`). The one in use is
   outlined.

   | Preset | Tip | Body | How close | Use it for |
   |---|---|---|---|---|
   | Light | 0.25 mm | 0.8 mm | far apart | small, light parts; marks come off easily |
   | Medium | 0.35 mm | 1.0 mm | normal | most models |
   | Heavy | 0.5 mm | 1.4 mm | close | big, heavy parts and anything that fell off before |

2. Pick where they go:
   - **Everywhere** — a full set: islands, lowest points, edges and every overhang steeper than
     the overhang angle.
   - **Islands only** — only the points that would print in mid-air. Few supports, for models that
     are mostly self-supporting.
3. Press **Auto supports** (`Ctrl+U`). With a model selected only that model gets supports,
   otherwise all of them. It runs in the background; a big model takes a few seconds. Right after,
   Copal checks the islands and says if all of them are held.

What the generator does, most important first:
- lifts the model to the lift height (Supports tab) plus the raft,
- finds **islands** from the layers and supports each one. An island in a tight spot gets a short,
  thin support, and one with no way down at all gets a short strut to the nearest part of the model,
- supports the **lowest points** of the model, the very first things that print,
- lines up supports along **edges**: the free rim of every overhang (a little inside the edge) and
  ridges that point down,
- covers the **overhangs** themselves with an even honeycomb grid, denser near the plate where the
  peel force is highest,
- bends a support around a part of the model below it, or lets it start on the model when there
  is no way down,
- lets close supports **share a trunk**: a support next to another leans over and joins it, so
  there are fewer feet and a smaller raft (**Let close supports share a trunk** in the Supports tab),
- makes tall supports thicker, because long thin ones sway,
- ties the trunks into a **lattice** (see below),
- puts a chamfered raft under all feet (**Raft** in the Supports tab).

The same model always gets the same supports. Supports you placed by hand stay when you run auto
supports again.

## Supports hold each other (0.3)
A single thin support bends. Tied to its neighbours it does not. With **Auto bracing** on (Supports
tab) every trunk is tied with diagonal braces to its three nearest neighbours, and all trunks of a
model hang together as one lattice: no group stands alone. Braces climb at 45°, alternate their
direction, and never go through the model. Trunks that stand close get a brace every
**bracing spacing** millimetres, trunks further apart get an unbroken zigzag.

## Hollow models and drain holes (0.3)
- Parts **inside a hollow** that would print in mid-air are found too. They get supports that
  stand on the floor of the hollow.
- A drain hole at the lowest point of a model takes away the very spot where the print starts.
  Copal sees what is left around the hole and supports that. No support stands in a hole.

So make the hollow and the holes first, then the supports. **Prepare for printing** does it in
that order.

## Overhangs in numbers (0.3)
**Overhangs** in the tools panel (or `overhangs` in the console) says how much of the model hangs
over at the overhang angle: the area, how many separate areas, the lowest points and the length of
the edges to hold. Faces that lie on the plate do not count, and are no longer red in the 3D view.

## Placing supports by hand
Choose the **Add** tool (`A`) and click on the model: a support goes exactly there. It becomes
part of what is already there:

- next to another support it **leans over and joins it** as a branch (**Join the nearest support**),
- otherwise it gets its own body down to the plate and is **tied to the trunks around it** with
  braces (**Tie to neighbours with braces**).

Both switches show under the tools while **Add** is chosen. The first support on a model that sits
on the plate lifts the model. Supports placed by hand stay when you run auto supports again.

## Selecting, resizing and deleting
Choose the **Select** tool (`B`) and click a support (it turns orange). **Shift+click** adds more.
Then:
- change **Body** and **Tip** diameter and press **Apply size**,
- press **Delete** (the key or the button) to remove them with their braces,
- **Esc** or **Deselect** clears the selection.

**Remove** (`Ctrl+Shift+U`) removes all supports of the selected model (or of all models).

## Islands check
**Check islands** (`I`) looks for parts of a layer that nothing holds and marks them with red dots in the
3D view. "All islands are supported" means the model is safe to slice. The check runs by itself
after auto supports and after **Prepare for printing**. It looks inside hollows and around drain
holes too. Where a red dot stays, add a support by hand.

## Supports and moving the model
- Moving the model (drag, gizmo or the Position fields) keeps its supports.
- Rotating, scaling or mirroring removes them, because they no longer fit, and the model goes back
  to the plate. Undo brings them back.
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
| Auto bracing, bracing spacing | the lattice of braces between trunks, and how far apart braces are on close trunks |
| Let close supports share a trunk | a support next to another joins its trunk |
| Support islands | supports under parts of a layer with nothing below |
| Manual bracing | braces for supports you add by hand |
| Auto parent | supports join a trunk next to them (automatic ones and ones added by hand) |
| Raft, raft thickness, raft margin | the plate under the supports |

Save your own mix as a preset with the **Supports presets** row.

## From the console or terminal
```
supports auto all          # auto supports for every model
supports 1                 # short for: supports auto 1
supports islands 1         # islands only, model 1
supports list              # how many supports each model has
overhangs all              # how much hangs over: area, lowest points, edges
islands all                # check islands
supports clear 1
supports.add id=1 x=10 y=-4 z=22     # one support at a point on the model
supports.remove id=1 sid=4,7
supports.resize id=1 sid=4 diameter=1.2 tip=0.5
```
`copal-cli run part.copal supports.auto id=1` does the same without opening the app.

## Planned
Full tree supports, support painting, a proximity check, STL export with supports.
