Your own resins for Copal
=========================

Every .toml file in this folder is one resin. Copal reads the folder while it runs: drop a file
here and it shows up in the resin list within a few seconds. Updates of Copal never touch it.

The easiest way to make a file: set the exposure and motion in Copal (Settings → Resin), then
"Save as my resin". You can also copy a file someone shared with you, or write one by hand:

    brand = "Anycubic"
    name = "ABS-Like Pro 2"
    color = "Grey"

    [[profiles]]
    printer = "elegoo/saturn-4-ultra-16k"     # `copal-cli printers` lists the ids
    layer_height_mm = 0.03
    exposure_s = 1.4
    bottom_layers = 5
    bottom_exposure_s = 29.0
    transition_layers = 5
    source = "my test prints"

Only `[[profiles]]` with a `printer` is required. Everything you leave out gets Copal's default.
Add one [[profiles]] section per printer. Waits, lift and retract use the names from Copal's
built-in resins, for example:

    rest_after_retract_s = 2.0
    lift_mm = [2.0, 1.0]          # two-stage lift: first stage, second stage (mm)
    lift_speed = [45.0, 180.0]    # mm/min

A file with the same `id` as a built-in resin (for example id = "anycubic/abs-like-pro-2-grey")
replaces the built-in one. Without an id the resin is "my/<file name>".
