# Calibration wizard

Resin settings depend on everything: printer, resin brand, **color**, batch, room temperature,
layer height, even the age of the screen. A database can only give a starting point.
So Ambra treats calibration as a first-class feature, not a hidden tool.

## Idea

> Print one small test, look at it, answer a few questions — get settings that fit **your** bottle.

## Flow

1. **Choose a printer and a resin.** Ambra fills a starting point from presets
   (or from the closest similar resin, marked as an estimate).
2. **Describe the bottle** (optional): color, batch number, opening date, room temperature.
3. **Print the exposure test.** One plate with several chips, each chip with a different
   exposure (for example 1.2 / 1.4 / 1.6 / 1.8 / 2.0 s). Takes ~15–30 minutes.
4. **Judge the result.** The wizard shows what to look at on each chip: fine pins, holes,
   positive/negative text, sharp edges. The user picks the best chip.
   Optional: take a photo with the phone — the app can help compare chips.
5. **Optional XY test.** Print a gauge, measure with calipers, enter the numbers → Ambra computes
   XY compensation and hole compensation.
6. **Save as "my resin".** A personal preset like
   `Anycubic ABS-Like Pro 2 · Grey · batch 2410 · 22 °C`, with the test date.

## Extras

- **Bottom exposure test** for plate adhesion (separate small test).
- **Re-check reminder:** when a new bottle or batch is used, suggest a quick re-test.
- **Share results** (opt-in): export a preset file to send as a GitHub issue / PR, so the
  community database grows from real prints.
- **Temperature note:** save room temperature with each test. Later we can suggest corrections
  when it gets colder or warmer (research item).

## What the user needs

- The printer and the resin.
- Good light and a phone camera (for optional photos).
- Digital calipers (only for the XY test).

## Milestones

- 0.2: exposure test generator + manual pick + save personal preset (Lite and CLI).
- 0.4: full wizard UI in the main app.
- 0.8: XY gauge, bottom exposure test, temperature notes.
