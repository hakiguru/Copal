<p align="center">
  <img src="assets/wordmark.png" width="432" alt="Copal">
</p>

<p align="center">
  <b>A free and open-source slicer for resin 3D printers.</b><br>
  Full-featured. No ads. No paywalls. No accounts.
</p>

<p align="center">
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-f07a12?style=flat-square"></a>
  <img alt="Status: planning" src="https://img.shields.io/badge/status-planning-b03a06?style=flat-square">
  <img alt="Platforms" src="https://img.shields.io/badge/platforms-Linux%20%7C%20Windows-fc9e26?style=flat-square">
  <img alt="Written in Rust" src="https://img.shields.io/badge/made%20with-Rust-702004?style=flat-square">
</p>

---

> [!NOTE]
> Copal is in the **planning stage**. There is nothing to download yet.
> Star or watch the repo to follow the progress.

## What is Copal?

Copal is a slicer for MSLA / LCD resin printers. It prepares 3D models for printing:
you place a model, add supports, and Copal turns it into a file your printer understands.

Good resin slicers today are closed, show ads, lock features behind a subscription,
or don't run on Linux at all. Copal wants to give everyone the full toolset for free,
on every machine, from a gaming PC to an old laptop.

*Copal* is young natural resin, still on its way to becoming amber.
The logo is a crystal of it.

## Three ways to use it

| | Edition | For whom |
|---|---|---|
| 🟠 | **Copal** | The full app. Modern interface, a big 3D viewport, Blender-style model editing. |
| 🟡 | **Copal Lite** | The same features in a simple, fast interface. Made for weak laptops with integrated graphics. |
| ⚫ | **Copal CLI** | Everything from the terminal. Scripts, batch slicing, servers, no graphics needed. |

All three share one engine, so a project made in one opens in the others.

## Planned features

**Prepare**
- Import STL, OBJ and 3MF
- Move, rotate, scale, mirror, duplicate, auto-arrange
- Automatic orientation
- Mesh repair, hollowing, drain holes

**Supports**
- Automatic supports: light, medium, heavy
- Manual add, move and delete
- Island, overhang and suction cup detection
- Rafts and support presets

**Edit**
- Blender-style edit mode: vertices, edges, faces
- Booleans, cutting with keys, text and primitives
- Undo and redo for everything

**Slice & export**
- Fast slicing on CPU or GPU, with anti-aliasing
- Layer-by-layer preview
- Export to `.goo`, `.ctb`, Photon Workshop formats, `.sl1s`, NanoDLP and more
- Built-in printer and resin presets
- Print time, resin use and cost estimate

## Platforms

| | Copal | Copal Lite | CLI |
|---|:---:|:---:|:---:|
| Windows (installer, portable) | ✅ | ✅ | ✅ |
| Linux AppImage | ✅ | ✅ | ✅ |
| Debian / Ubuntu (`.deb`) | ✅ | ✅ | ✅ |
| Fedora / openSUSE (`.rpm`) | ✅ | ✅ | ✅ |
| Arch Linux (AUR) | ✅ | ✅ | ✅ |
| Flatpak | ✅ | ✅ | — |

*All planned.*

## Roadmap

The full plan lives in [`docs/ROADMAP.md`](docs/ROADMAP.md).
The tech design is in [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).
The feature checklist is in [`docs/FEATURES.md`](docs/FEATURES.md).

- [ ] **0.1** Engine: load a model, slice it, export `.goo` and `.ctb` from the terminal
- [ ] **0.2** Copal Lite: first app with a 3D view, layer preview and resin calibration
- [ ] **0.3** Automatic and manual supports: grid, inline, projection, painting, bracing, rafts
- [ ] **0.4** Full Copal app and Windows builds
- [ ] **0.5** Orientation, hollowing, arranging, mesh repair, multi-plate, one-click prepare
- [ ] **0.6** Blender-style editing, booleans, cut with keys
- [ ] **0.7** Large printer and resin database, all major file formats
- [ ] **0.8** GPU slicing, sending to printer over Wi-Fi, advanced calibration

## Calibrate your own resin

Resin settings change with the color, the batch and even the room temperature.
Copal includes a calibration wizard: print one small test, pick the best chip, and save
settings made for **your** bottle. Built-in presets are just the starting point.
- [ ] **1.0** Stable release

## Contributing

Ideas, bug reports and printer tests are very welcome. Open an issue.
If you own a resin printer that is not on the list, your help is gold.

## License

Copal is released under the [MIT License](LICENSE). Use it, change it, sell it, ship it —
anything you like, just keep the copyright notice.

---

<details>
<summary><b>🇷🇺 На русском</b></summary>

**Copal** — бесплатный слайсер с открытым кодом для фотополимерных (MSLA/LCD) 3D-принтеров.
Полный набор функций, без рекламы, без платных подписок и без аккаунтов.

- **Copal** — полная версия: современный интерфейс и редактирование моделей в стиле Blender.
- **Copal Lite** — те же функции в простом и быстром интерфейсе для слабых ноутбуков со встроенной графикой.
- **Copal CLI** — всё то же самое из терминала, без графики.

Сборки для Windows и Linux: AppImage, deb, rpm, AUR, Flatpak.
Проект на стадии планирования. План — в [`docs/ROADMAP.md`](docs/ROADMAP.md).

</details>
