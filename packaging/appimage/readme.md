# ChipDX for Linux

The ChipDX AppImage includes the game, level editor and five community levelsets.
Download one file to play on an x86_64 Linux desktop.

## Download and play

1. Download `ChipDX-x86_64.AppImage` from the [latest release](https://github.com/CasualX/chipdx/releases/latest).
2. Make it executable, either in your file manager's permissions or with the command below.
3. Launch it and choose a levelset to begin playing.

```sh
chmod +x ChipDX-x86_64.AppImage
./ChipDX-x86_64.AppImage
```

You can also download the accompanying `.sha256` file and verify your download:

```sh
sha256sum -c ChipDX-x86_64.AppImage.sha256
```

## Level editor

Launch the editor from a terminal, optionally supplying a level JSON file:

```sh
./ChipDX-x86_64.AppImage --editor
./ChipDX-x86_64.AppImage --editor path/to/level.json
```

Relative level paths are resolved from the directory where you launch the image.
Use **F2** to open a level and **F5** to save it in a location of your choice.
The editor shares the game's configuration. See the
[level editor controls](../../readme.md#level-editor) for more shortcuts.

## System requirements

- An x86_64 Linux desktop with glibc 2.31 or later.
- Working graphics drivers supporting OpenGL 3.3, with X11 or Wayland.
- GTK 3 and libnotify for the editor's file dialogs. These are usually available
  on desktop installations; on Debian/Ubuntu, install `libgtk-3-0` (or
  `libgtk-3-0t64` on newer releases) and `libnotify4` if they are missing.

| Distribution / environment | Compatibility |
| --- | --- |
| Ubuntu 20.04, 22.04, 24.04; Debian 11, 12, 13 | Targeted desktop releases, with X11 and Wayland support |
| Newer Ubuntu/Debian and Ubuntu-based Linux Mint | Expected to work with the requirements above |
| Fedora, Arch, openSUSE and other glibc desktops | Expected to work with the requirements above; not every distribution is tested |
| NixOS | Use a suitable environment such as `appimage-run` |
| Alpine / musl, glibc older than 2.31, ARM or 32-bit x86 | Not supported by this AppImage |

## Configuration, saves and additional levels

Your files are stored in `~/.local/share/chipdx/` by default:

| File or folder | Purpose |
| --- | --- |
| `chipdx.ini` | Game and editor settings |
| `save/` | Game progress and records |
| `levelsets/` | Additional levelsets: put downloaded `.dat` files here |

If you have set `XDG_DATA_HOME`, the location is `$XDG_DATA_HOME/chipdx/` instead.
Levels you create in the editor are saved wherever you choose in its save dialog.

To update, download the new AppImage and replace the old file. Your configuration,
saves and additional levelsets are preserved. Keep the AppImage at its current
location while the game or editor is running.

## Troubleshooting

If you cannot launch the image because FUSE is unavailable, use:

```sh
./ChipDX-x86_64.AppImage --appimage-extract-and-run
./ChipDX-x86_64.AppImage --appimage-extract-and-run --editor
```

If native Wayland startup fails, you can try Xwayland if it is installed and
available in your desktop session:

```sh
env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET ./ChipDX-x86_64.AppImage
```

When [reporting a problem](https://github.com/CasualX/chipdx/issues), include your
Linux distribution, graphics card/driver, whether you use X11 or Wayland, and any
error messages shown when launching from a terminal.
