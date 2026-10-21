> [!NOTE]
> 🚧 This page describes a planned feature. It arrives in **Copal 0.2**. Details may change.


Resin settings depend on your printer, the resin **color**, the **batch**, and even the room
temperature. A preset is only a starting point. The calibration wizard finds settings for
**your** bottle.

## What you need
- Your printer and the resin
- Good light (a phone camera helps)
- Digital calipers — only for the optional XY test

## Steps
1. **Choose printer and resin.** Copal fills in a starting point.
2. **Describe the bottle** (optional): color, batch number, opening date, room temperature.
3. **Print the exposure test.** One small plate with chips at different exposures. ~15–30 min.
4. **Pick the best chip.** The wizard shows what to look at: fine pins, holes, small text, edges.
5. **XY test** (optional, 0.8). Print a gauge, measure it, enter the numbers.
6. **Save.** You get a personal preset, for example
   `ABS-Like Pro 2 · Grey · batch 2410 · 22 °C`.

## Tips
- Re-test when you open a new bottle or the room gets much colder or warmer.
- Shake the bottle well before every test.
- You can share your result to help others — see [[Add a printer or resin|Adding-Printers-and-Resins]].

From the terminal: `copal-cli calibrate --printer <id> --resin <id>`.
