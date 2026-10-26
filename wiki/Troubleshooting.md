> [!NOTE]
> 🚧 This page describes a planned feature. It arrives in **Copal 0.3**. Details may change.


## Copal does not start or the window is black
Your graphics driver may be too old for the full app. Install **Copal Lite**, or start Copal with
`copal --renderer gl`.

## Very slow on a big model
- Use Copal Lite.
- Turn on **Settings → Viewport → Simplify large models**.
- Slicing uses all CPU threads; close other heavy apps.

## The printer does not show the file
- Check that the printer and the format match (Settings → Printer).
- Some printers need the file in the USB root folder and a short file name.

## Print failed
- Check islands and supports before slicing.
- Run the [[Calibration wizard|Calibration-Wizard]] for your resin.
- Open an issue with the project file and a photo.

## Reporting a bug
[Open an issue](https://github.com/hakiguru/Copal/issues) and attach: Copal version, OS, the `.copal` project and the log from
**Help → Open log folder**.
