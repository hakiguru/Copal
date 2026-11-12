<p align="center">
  <img src="https://raw.githubusercontent.com/hakiguru/Copal/main/assets/wordmark.png" width="432" alt="Copal">
</p>

<p align="center">
  <b>The free and open-source slicer for resin 3D printers.</b><br>
  Full-featured. No ads. No paywalls. No accounts.
</p>

---

> [!NOTE]
> **Copal 0.1 slices!** Import STL / OBJ / 3MF, place models with handles or by dragging them over
> the plate, set up exposure and lift, look through the layers and export an Elegoo `.goo` file.
> Every export goes to the print history with a copy of the project. **0.2** adds auto and manual
> supports with braces and a raft, hollowing, drain holes, your own resins and panels that fold
> away. **0.3** prepares a model in one click: auto orientation, hollowing with drain holes that
> are checked, supports tied into a lattice. It also brings your own printers, tidy tool panels
> and hotkeys for almost everything. **0.4** tilts models on all axes when it orients them, leaves
> no support stubs on the model, adds pads and a grid as a base, a layer slider, the weight and
> price of every print, and keys you can change.
> Pages for planned features say which version brings them.

## Welcome

Copal prepares 3D models for MSLA / LCD resin printers. You place a model, add supports,
slice it, and get a file your printer understands — `.goo`, `.ctb` and many more.

This wiki is the user guide. It grows together with the app.

## Start here

| Page | What it covers |
|---|---|
| 📦 **[Installation](https://github.com/hakiguru/Copal/wiki/Installation)** | Get Copal for Windows or Linux |
| 🧭 **[Which edition?](https://github.com/hakiguru/Copal/wiki/Which-Edition)** | Copal, Copal Lite or the CLI — which one fits your computer |
| 🖨️ **[Your first print](https://github.com/hakiguru/Copal/wiki/First-Print)** | From an STL file to a finished print, step by step |
| 🎯 **[Calibration wizard](https://github.com/hakiguru/Copal/wiki/Calibration-Wizard)** | Find the right exposure for **your** resin bottle |

## User guide

| Prepare | Supports & slicing | Reference |
|---|---|---|
| [Interface overview](https://github.com/hakiguru/Copal/wiki/Interface-Overview) | [Supports](https://github.com/hakiguru/Copal/wiki/Supports) | [Hotkeys](https://github.com/hakiguru/Copal/wiki/Hotkeys) |
| [Layout: place, rotate, scale](https://github.com/hakiguru/Copal/wiki/Layout) | [Hollowing and drain holes](https://github.com/hakiguru/Copal/wiki/Hollowing-and-Drain-Holes) | [Command line](https://github.com/hakiguru/Copal/wiki/Command-Line) |
| [Prepare: ready in one click](https://github.com/hakiguru/Copal/wiki/Prepare) | [Slice and export](https://github.com/hakiguru/Copal/wiki/Slice-and-Export) | [Project files](https://github.com/hakiguru/Copal/wiki/Project-Files) |
| [Auto orientation](https://github.com/hakiguru/Copal/wiki/Auto-Orientation) | | |
| [Editing models](https://github.com/hakiguru/Copal/wiki/Editing) | [Print history](https://github.com/hakiguru/Copal/wiki/Print-History) | [Resin presets](https://github.com/hakiguru/Copal/wiki/Resin-Presets) |

## Printers and resins

- [[Supported printers|Supported-Printers]] — which printers work and how well they are tested
- [[File formats|File-Formats]] — `.goo`, `.ctb`, Photon Workshop formats and others
- [[Resin presets|Resin-Presets]] — built-in starting points and your own presets
- [[Add a printer or resin|Adding-Printers-and-Resins]] — help the database grow

## Help

- [[FAQ]]
- [[Troubleshooting]]
- [[Running Copal Lite without a desktop|Running-Without-Desktop]]

## For developers

- [[Building from source|Building-from-Source]]
- [[Architecture]]
- [[Contributing]]
- [[Release process|Release-Process]]
- [Roadmap](https://github.com/hakiguru/Copal/blob/main/docs/ROADMAP.md)

---

<details>
<summary><b>🇷🇺 На русском</b></summary>

**Copal** — бесплатный слайсер с открытым кодом для фотополимерных (MSLA/LCD) 3D-принтеров.
Без рекламы, без подписок и без аккаунтов. Работает на Windows и Linux, есть лёгкая версия
для слабых ноутбуков и полноценная работа через терминал.

С версии 0.1 Copal нарезает модели в файлы `.goo` для Elegoo: импорт STL / OBJ / 3MF, гизмо для
перемещения, поворота и масштаба, перетаскивание модели по столу, настройки смолы и печати,
предпросмотр слоёв и история печати с копией проекта. В 0.2 появились авто- и ручные поддержки с распорками и рафтом, опустошение, дренажные отверстия и свои пресеты смол.
В 0.3 модель готовится к печати одной кнопкой: автоориентация, опустошение с дренажными
отверстиями (с проверкой, что они открывают полость) и поддержки, связанные в решётку. Ручные
поддержки присоединяются к уже стоящим. Ещё появились свои шаблоны принтеров, аккуратные панели
инструментов и горячие клавиши почти на все действия (список по F1).
В 0.4 автоориентация наклоняет модель по всем осям (чемодан больше не ставится на торец),
поддержки не оставляют «пеньков» на модели, появились основания «площадки» и «сетка», ползунок
слоёв, вес и стоимость печати после нарезки и переназначаемые горячие клавиши.
Вики пока на английском; русский интерфейс и страницы появятся позже.

</details>
