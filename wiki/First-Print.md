> [!NOTE]
> Every step works since **Copal 0.2**. Auto orient (step 3) arrives in 0.3; until then turn the
> model by hand.


A full walkthrough from a model file to a print.

## 1. Pick your printer and resin
Choose them in the top bar. If your resin is not on the list, pick the closest one and run the
[[Calibration wizard|Calibration-Wizard]] later.

## 2. Import a model
Drop an STL, OBJ or 3MF file on the window, or press **Import models…** (Ctrl+I).

## 3. Orient and place
Rotate the model with the **Rotate** handles (`R`) and drag it over the plate. Tilting models
30–45° usually prints better. **Auto orient** comes in 0.3. See [[Layout]].

## 4. Add supports
Open the **Prepare** step, pick **Light**, **Medium** or **Heavy** and press **Auto supports**.
Press **Check islands**: it should say "All islands are supported". Add missing supports with the
**Add** tool.
See [[Supports]].

## 5. Hollow (optional)
Large solid models waste resin and can crack. Hollow them and add drain holes.
See [[Hollowing and drain holes|Hollowing-and-Drain-Holes]].

## 6. Slice and check
Open the **Export** step, press **Layers** and scroll through them to look for problems.

## 7. Export and print
Press **Slice & export…**, save the `.goo` file to a USB stick. Afterwards rate the print in the
[[print history|Print-History]].
See [[Slice and export|Slice-and-Export]].
