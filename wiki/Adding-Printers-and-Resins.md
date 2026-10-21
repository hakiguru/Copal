> [!NOTE]
> 🚧 This page describes a planned feature. It arrives in **Copal 0.7**. Details may change.


The printer and resin database is open. Anyone can add to it.

## Add a printer
1. Copy a file from `presets/printers/` in the [repository](https://github.com/hakiguru/Copal).
2. Fill in the screen resolution, display size, build height and file format from the
   **manufacturer's spec page**. Put the link in `[sources]`.
3. Open a pull request, or attach the file to an issue.

## Confirm a printer works
Print a test model and open a **Printer test report** issue with a photo. Your printer becomes ✅ Stable.

## Add a resin
1. Run the [[Calibration wizard|Calibration-Wizard]].
2. Use **Share result** to export your preset.
3. Attach it to a **Resin test report** issue.

## Rules
- Every value needs a source: a link or your own test.
- Don't copy settings databases from other slicers.
