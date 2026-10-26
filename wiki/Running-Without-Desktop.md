> [!NOTE]
> 🚧 This page describes a planned feature. It arrives in **Copal 0.3**. Details may change.


Copal Lite can run on a system with **no desktop environment**, for example a minimal Linux install
on an old laptop.

## Option 1: Lite in a kiosk compositor
`cage` is a tiny Wayland compositor that runs one app full screen, straight from a text console.

```sh
sudo apt install cage      # or: sudo pacman -S cage / sudo dnf install cage
cage copal-lite
```

Works with integrated graphics. Without a GPU it uses software rendering (Mesa llvmpipe).

## Option 2: TUI
No graphics at all? Use the text interface:

```sh
copal-cli tui my-project.copal
```

Terminals with sixel or kitty graphics show layer previews.

## Option 3: CLI only
Everything is available as commands — see [[Command line|Command-Line]].
