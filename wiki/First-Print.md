A full walkthrough from a model file to a print. Every step works since **Copal 0.3**.

## 1. Pick your printer and resin
Choose them in the top bar. If your resin is not on the list, pick the closest one and run the
[[Calibration wizard|Calibration-Wizard]] later.

## 2. Import a model
Drop an STL, OBJ or 3MF file on the window, or press **Import models…** (Ctrl+I).
Drag it to where you want it on the plate. See [[Layout]].

## 3. Prepare it
Open the **Prepare** step (`2`) and press **Prepare for printing** (`Ctrl+P`). Copal turns the
model the best way up, adds supports with braces and a raft, and checks that nothing is left in
mid-air. For a big, bulky model tick **Hollow** first: it is hollowed and gets drain holes too.

The console says what was done. A red line names what still needs you, for example an island
that could not be reached: add a support there with the **Add** tool.

Want to decide yourself? The same panel does every step on its own:
[[Auto orientation|Auto-Orientation]], [[Hollowing and drain holes|Hollowing-and-Drain-Holes]],
[[Supports]]. See [[Prepare]].

## 4. Slice and check
Open the **Export** step (`3`), press **Layers** and scroll through them to look for problems.
A hollow model shows its walls and its drain holes there.

## 5. Export and print
Press **Slice & export…**, save the `.goo` file to a USB stick. Afterwards rate the print in the
[[print history|Print-History]].
See [[Slice and export|Slice-and-Export]].

## A big model, step by step
What to do for a bulky part, such as a box, a bust or a helmet:

1. **Prepare → Hollow** ticked, wall 2.5–3 mm.
2. Support preset **Heavy**: thick, close supports.
3. **Prepare for printing**. The part ends up tilted, standing on its narrow side, with two drain
   holes near the plate.
4. Read the console. "every island is supported" and "2 holes open the hollow" are what you want.
5. In **Export → Layers** look at a layer near the bottom: the holes must be open.
6. After printing, wash the inside through the holes and let it dry before curing.
