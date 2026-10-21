# Studying other slicers

We want Ambra to be at least as good as LycheeSlicer and Chitubox. We learn from them the
legal way: **as a black box**. We look at what they do, never at how their code does it.

## Allowed

- Using the apps normally (Lite, and Plus during its free 30-day trial) and taking screenshots
  and screen recordings of the UI.
- Reading their public docs, changelogs, help center, forum posts and YouTube tutorials.
- Slicing **our own** test models in them and studying the output: exported STL/OBJ with supports,
  sliced printer files, layer images, print time and resin estimates.
- Measuring results: support count, tip size, spacing, angles, raft shape, hollow wall thickness.
- Comparing real prints made from their files and from ours.

## Not allowed

- Unpacking, decompiling or disassembling their installers or binaries (`.deb`, `.exe`, `.asar`, …).
- Copying their built-in printer or resin databases, icons, images or texts.
- Unlocking paid features without paying.

## Test model set

Kept in `tests/samples/research/` (only models we made or that are CC0):

| Model | What it tests |
|---|---|
| Flat plate 50×50×2 mm | flat overhang, suction |
| Sphere 30 mm | islands at the bottom, round overhang |
| Hollow cube with no holes | resin trap, suction cup |
| Bridge / arch | long overhangs |
| Thin pillar forest | small islands, tip size |
| Miniature figure (CC0) | real-world mix |
| Calibration chips | exposure, XY accuracy |

## How a study is recorded

One file per topic in `docs/research/`, e.g. `auto-supports-lychee.md`:

1. Settings used (screenshot).
2. Input model and its orientation.
3. Measured output: numbers in a table, pictures.
4. What we conclude: **the behaviour Ambra should match or beat**, written as a goal,
   not as an algorithm copied from them.

Our algorithms are then designed from these goals plus public research papers and our own tests.
