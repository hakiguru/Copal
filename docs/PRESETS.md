# Printer and resin presets

Presets are plain TOML files in `presets/`. Anyone can add a printer or a resin with a pull request.

## Where the data comes from

- Manufacturer spec pages and technical data sheets (TDS).
- Manufacturer exposure charts (most resin brands publish settings per printer).
- Real test prints by users (calibration wizard results, opt-in).
- **Not** extracted from LycheeSlicer, Chitubox or other closed slicers' databases,
  and not copied from GPL/AGPL projects' profile files.
- Every value has a source: a link, or "tested by @user on <date>".

Built-in resin presets are **starting points**. Exposure really depends on the printer, the resin
color, the batch and the room temperature, so the real settings come from the user's own
calibration (`docs/CALIBRATION.md`) and are saved as a personal preset.

## Personal resin presets

Saved in the user config folder, never overwritten by updates:

```toml
id = "my/abs-like-pro-2-grey-2410"
based_on = "anycubic/abs-like-pro-2-grey"
batch = "2410"
opened = 2026-10-01
room_temp_c = 22

[[profiles]]
printer = "elegoo/saturn-4-ultra-16k"
layer_height_mm = 0.05
bottom_exposure_s = 20.0
exposure_s = 1.5
calibrated = 2026-10-21      # date of the exposure test
```

## Printer preset

`presets/printers/elegoo/saturn-4-ultra-16k.toml`

```toml
id = "elegoo/saturn-4-ultra-16k"
brand = "Elegoo"
model = "Saturn 4 Ultra 16K"
status = "stable"            # stable | experimental

[screen]
resolution = [15120, 6230]   # pixels, X × Y  (example — verify from spec)
size_mm = [211.68, 118.37]   # display area   (example — verify from spec)
mirror_x = false
greyscale_bits = 8

[build]
height_mm = 220              # (example — verify from spec)

[file]
format = "goo"
thumbnails = [[116, 116], [290, 290]]

[motion]
supports_tsmc = true         # two-stage lift / retract
default_lift_mm = 5
default_lift_speed_mm_min = 120

[network]
protocol = "sdcp"

[sources]
spec = "https://www.elegoo.com/..."
```

## Resin preset

`presets/resins/anycubic/abs-like-pro-2-grey.toml`

```toml
id = "anycubic/abs-like-pro-2-grey"
brand = "Anycubic"
name = "ABS-Like Resin Pro 2"
color = "Grey"
type = "abs-like"            # standard | abs-like | water-washable | plant-based | tough |
                             # flexible | castable | dental | high-temp | 8k-detail
wavelength_nm = [365, 405]

[properties]                 # from the TDS, all optional
density_g_ml = 1.10          # (example — fill from TDS)
viscosity_mpa_s = 200        # (example — fill from TDS)
shrinkage_pct = 0.0
shore_hardness_d = 0
tensile_strength_mpa = 0
elongation_pct = 0
price_per_kg = 0             # user can override

# Exposure settings per printer and layer height.
[[profiles]]
printer = "elegoo/saturn-4-ultra-16k"
layer_height_mm = 0.05       # to confirm
bottom_layers = 4            # to confirm
bottom_exposure_s = 20.0     # tested by the project owner
exposure_s = 1.5             # tested by the project owner
source = "tested by @hakiguru, 2026"
```

## Rules

- IDs are `brand/model` in lowercase kebab-case and never change once released.
- A missing exposure profile for a printer → Copal suggests running an exposure test and
  estimates from similar printers (marked as an estimate).
- Users can save their own presets; they live in the user config folder and override built-ins.
- CI validates every preset file (schema, ranges, source present).

## Initial targets

- **Printers (0.7):** the 30+ most popular models from Elegoo, Anycubic, Phrozen, Creality,
  Uniformation, Prusa, plus whatever the community adds.
- **Resins (0.7):** 50+ resins: Elegoo, Anycubic, Siraya Tech, Phrozen, Sunlu, eSun, Jamg He,
  3DMaterials, Liqcreate, Formfutura and others.
