Your own printers for Copal
===========================

Every .toml file in this folder is one printer. Copal reads the folder while it runs: drop a file
here and the printer shows up in the printer list within a few seconds. Updates of Copal never
touch it.

The easiest way to make a file: choose a printer close to yours in Copal, then
"Save as my printer" (Settings → Print) or `presets save printer <name>` in the console, and
change the numbers in the file. You can also copy a file someone shared with you, or write one:

    brand = "Elegoo"
    model = "Saturn 4 Ultra 16K"

    [screen]
    resolution = [15120, 6230]       # pixels, X and Y
    size_mm = [211.68, 118.37]       # size of the display area
    mirror_x = false
    mirror_y = true                  # flip the image top to bottom

    [build]
    height_mm = 220.0

    [file]
    format = "goo"                   # the file the printer reads
    format_version = "V3.0"
    thumbnails = [[116, 116], [290, 290]]
    anti_aliasing_levels = 8

    [motion]
    supports_tsmc = true             # two-stage lift and retract

    [sources]
    values = "where the numbers come from"

Only [screen] (resolution, size_mm), [build] and [file] are required. Without an id the printer
is "my/<file name>"; brand and model are taken from the file name when they are missing.

A file with the same `id` as a built-in printer (for example id = "elegoo/saturn-4-ultra-16k")
replaces the built-in one.

Copal writes .goo files today. A printer with another format can be laid out and prepared, and
it exports as soon as Copal learns that format.

Resins live next door, in the "resins" folder.
