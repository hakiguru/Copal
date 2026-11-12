How a model is turned decides if it prints at all. Copal can find a good orientation for you.

## Why orientation matters
Every layer has to peel off the film at the bottom of the vat. The bigger the layer, the harder
it pulls on the supports and on the plate. A print comes off the plate, or tears off its supports,
when one layer pulls harder than they can hold. So a good orientation has:

- **small layers** — the model stands on its narrow side, not on its biggest face,
- **layers that grow slowly** — no big flat face level with the plate that appears all at once,
- **little that hangs over** — fewer supports, fewer marks to sand off,
- **not too much height** — a tall print takes longer.

These pull in different directions, so you choose what matters most.

## Auto orient (0.3)
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
Copal tries about 700 directions for "down" and gives each a score from four numbers:

- the area of the **biggest layer**,
- the most a layer **grows** over one millimetre of height (a flat face that looks down shows up
  here),
- the **area that hangs over**, with level faces counted in full and faces near the overhang angle
  counted less,
- the **height**.

The same model always gets the same answer. It is a suggestion: turn the model further with the
rotate handles if a face you care about ended up on the support side.

## Tips
- After **Strong**, a box-like part stands almost upright with a slight tilt. That is right: its
  layers are as small as they can be.
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
