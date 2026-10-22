> [!NOTE]
> Steps 1–3 and 6–7 work in **Copal 0.1**. Supports (step 4) arrive in 0.3 and hollowing (step 5)
> in 0.2 — until then add supports in another tool or print models that need none.


A full walkthrough from a model file to a print.

## 1. Pick your printer and resin
Choose them in the top bar. If your resin is not on the list, pick the closest one and run the
[[Calibration wizard|Calibration-Wizard]] later.

## 2. Import a model
Drop an STL, OBJ or 3MF file on the window, or press **Import models…** (Ctrl+I).

## 3. Orient and place
Rotate the model with the **Rotate** handles (`R`) and drag it over the plate. Tilting models
30–45° usually prints better. **Auto orient** comes in 0.2. See [[Layout]].

## 4. Add supports
Open the **Supports** step and press **Auto supports**. Check the island warnings.
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
