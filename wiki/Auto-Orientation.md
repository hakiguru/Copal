How a model is turned decides if it prints at all. Copal can find a good orientation for you.

## Why orientation matters
Every layer has to peel off the film at the bottom of the vat. The bigger the layer, the harder
it pulls on the supports and on the plate. A print comes off the plate, or tears off its supports,
when one layer pulls harder than they can hold. So a good orientation has:

- **small layers** — not on its biggest face,
- **layers that grow slowly** — no big flat face level with the plate that appears all at once,
- **little that hangs over** — fewer supports, fewer marks to sand off,
- **a steady stand** — a tall model on a narrow end sways on its supports with every peel and
  breaks off; a model that leans over a wide footprint is held well,
- **no faces standing straight up** — the lower edge of such a face is a level line that appears
  in one layer and needs a row of supports. A model tilted on two axes has none,
- **not too much height** — a tall print takes longer.

These pull in different directions, so you choose what matters most.

## Auto orient (0.3, tilts on all axes since 0.4)
Select a model and press **Auto orient** (`Ctrl+T`). The button is in **Layout** (under Transform)
and in **Prepare → 1 · Orientation**, where you also pick the goal:

| Goal | What it aims for | Use it for |
|---|---|---|
| **Balanced** | small layers that grow slowly, few overhangs, not too tall | most models |
| **Strong** | the smallest layers of all | big, heavy parts that came off the plate before |
| **Clean** | the least area that hangs over | parts where support marks must be few |
| **Low** | the lowest print | when print time matters most |

The console says what changed:

```
[1] biggest layer 2593 → 2112 mm², overhangs 4133 → 3394 mm², height 73 → 97 mm
```

The model stays where it was on the plate. Its supports are removed, because they were made for
the old orientation; drain holes and the hollow turn with the model. `Ctrl+Z` turns it back.

Only orientations that fit your printer are tried. A model that fits no way round gives an error:
scale it down or cut it.

## What Copal looks at
Copal tries about 700 directions for "down" — every way the model can be turned, over all three
axes — and gives each a score from these numbers:

- the area of the **biggest layer**,
- the most a layer **grows** over one millimetre of height (a flat face that looks down shows up
  here),
- the **area that hangs over**, with level faces counted in full and faces near the overhang angle
  counted less,
- how **slender** the model stands: its height against the area it covers on the plate,
- the area of faces that **stand straight up**,
- the **height**.

Then it turns the model around the vertical axis so its long side lies along the long side of the
plate.

When the project hollows models (Hollow is ticked, or the model is hollow already), only the walls
count as the area of a layer: the inside of a hollow does not pull on the film.

The same model always gets the same answer. It is a suggestion: turn the model further with the
rotate handles if a face you care about ended up on the support side.

## Tips
- A flat box such as a suitcase comes out leaning back by about 35° and turned a little to the
  side, so a corner touches first and no face is level or upright. That is the classic hand-made
  orientation for resin.
- **Strong** cares most about small layers and may stand a part more upright.
- Keep the side with fine detail away from the plate: supports go on the side that looks down.
- A big part still pulls hard. Hollow it ([[Hollowing and drain holes|Hollowing-and-Drain-Holes]])
  and use the **Heavy** support preset ([[Supports]]).

## From the console or terminal
```
orient                     # every model, balanced
orient 1 strong            # model 1, the smallest layers
orient all supports        # the least overhang (the Clean goal)
orient 1 fast              # the lowest print (the Low goal)
object.orient id=1 goal=balanced
```
