## The print came off the plate, or tore off its supports
One layer pulled harder on the film than the plate and the supports could hold.

- Turn the model: **Prepare → 1 · Orientation → Strong → Auto orient**. It then stands on its
  narrow side and every layer is as small as it can be. See [[Auto orientation|Auto-Orientation]].
- Hollow big parts and give them drain holes. A hollow without holes sucks itself onto the film.
  The line under **3 · Drain holes** must say the holes open the hollow.
  See [[Hollowing and drain holes|Hollowing-and-Drain-Holes]].
- Use the **Heavy** support preset and keep the raft on. See [[Supports]].
- Press **Check islands**: no red dots may be left.
- Use more bottom layers or a longer bottom exposure (Print tab), and level the plate.
- Lift more slowly (Print tab → lift speed). Cold resin is thick: warm the room.

## Resin is trapped inside a hollow print
The hollow had no hole, or a hole did not reach it. Run `holes check` in the console, or read the
line under **3 · Drain holes**. Each separate hollow needs two holes.

## A support is missing where I need one
Choose **Add** (`A`) in **Prepare → 4 · Supports** and click on the model. The new support joins
the ones around it. Supports you add by hand stay when you run auto supports again.

## Copal does not start or the window is black
Your graphics driver may be too old for the full app. Install **Copal Lite**: it runs on old
graphics and without a graphics card at all.

## Very slow on a big model
- Use Copal Lite.
- Slicing, hollowing and supports use all CPU threads; close other heavy apps.

## The printer does not show the file
- Check that the printer in the top bar is your printer.
- Some printers need the file in the USB root folder and a short file name.

## Print failed in another way
- Run the [[Calibration wizard|Calibration-Wizard]] for your resin.
- Open an issue with the project file and a photo.

## Reporting a bug
[Open an issue](https://github.com/hakiguru/Copal/issues) and attach: the Copal version, your
system, the `.copal` project and what the console said.
