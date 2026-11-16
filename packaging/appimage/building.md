# Building the Linux AppImage

The [AppImage user guide](readme.md) covers installation, compatibility and usage.
This document describes packaging, release artifacts and validation.

## GitHub Actions

Run **Build Binaries** (`workflow_dispatch`) on the branch containing the changes.
The `chipdx-appimage-x86_64` artifact contains `ChipDX-x86_64.AppImage` and
`ChipDX-x86_64.AppImage.sha256`. Wait for the Linux startup checks to pass before
replacing both files on the GitHub release.

The image includes `chipplay`, `chipedit`, shared assets and five curated
community levelsets. `AppRun` launches the game by default and the editor when
passed `--editor`. It resolves relative editor level paths before changing to
the writable user-data directory.

## Compatibility policy

Builds run inside Ubuntu 20.04, independently of the Actions runner. The ABI
baseline is **glibc 2.31**, x86_64 with baseline x86-64 CPU instructions.
`check-abi.py` audits packaged ELF files and rejects higher glibc requirements.
It also rejects libraries reserved for the host desktop, including the names
recorded in `usr/share/chipdx/host-libraries.txt`.

The host provides glibc, OpenGL/EGL, graphics drivers, Wayland, xkbcommon,
keyboard/Compose data and the editor's GTK 3/libnotify stack. On Debian/Ubuntu,
the desktop runtime packages include `libegl1`, the appropriate graphics driver,
`libwayland-client0`, `libwayland-cursor0`, `libwayland-egl1`, `libxkbcommon0`,
`libxkbcommon-x11-0`, `xkb-data`, `libgtk-3-0` (or `libgtk-3-0t64`) and `libnotify4`.
These compatibility targets are independent of distribution security lifecycles.

linuxdeploy bundles the remaining dependencies, including audio libraries.
GTK's full dependency stack (including text rendering and X11 client libraries)
comes from the host; its library names are recorded in the image's
`usr/share/chipdx/host-libraries.txt` and checked by the package audit.
ALSA/PulseAudio connect to the host audio system. Wayland libraries
must stay with the host graphics drivers: bundling Ubuntu 20.04's client library
breaks newer Mesa EGL drivers that require `wl_proxy_marshal_flags` (issue #4).
Using the host xkbcommon also avoids parsing newer Compose tables with an old
keysym database.

CI uses software OpenGL and null audio, with Xvfb for X11 and headless Weston
for Wayland. Both the game and editor are checked. The Wayland check unsets
DISPLAY and verifies that each application submits
native Wayland buffers, accepting both `interface@id` and `interface#id` debug
formats. Debian 11 jobs use Debian's signed archive repositories because the
live Bullseye security indexes reference packages removed after its LTS ended.
Expiry checks are disabled only for the archived security repository; package
signature verification remains enabled.
Linux builds prefer EGL with an sRGB surface, falling
back to GLX on X11. Real GPU drivers (including NVIDIA), desktop integration,
sound output, gamepads and interactive gameplay still need manual testing.
Passing startup checks does not guarantee every driver/compositor combination.

The linuxdeploy release and the statically linked AppImage runtime are pinned
and checksum-verified. linuxdeploy includes the AppImage output tool. It runs
using extract-and-run, so building needs neither FUSE nor a privileged container.

## Building locally

From the repository root, with Docker installed:

```sh
docker run --rm -v "$PWD:/src" -w /src ubuntu:20.04 bash -c '
  bash packaging/appimage/dependencies.sh
  curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
  . "$HOME/.cargo/env"
  bash packaging/appimage/build.sh
'
```

The result is in `target/appimage/dist/`. Builds use `target/appimage-build/`
to avoid reusing binaries compiled on a newer host. Before rebuilding, remove
`target/appimage/ChipDX.AppDir`. The script intentionally refuses to build on a
different glibc baseline; packaging a binary built on a recent host would defeat
the compatibility target.

## Startup checks

With Xvfb, xauth, Weston and the desktop runtime packages installed:

```sh
bash packaging/appimage/smoke-test.sh target/appimage/dist/ChipDX-x86_64.AppImage x11
bash packaging/appimage/smoke-test.sh target/appimage/dist/ChipDX-x86_64.AppImage wayland
bash packaging/appimage/smoke-test.sh target/appimage/dist/ChipDX-x86_64.AppImage x11 editor
bash packaging/appimage/smoke-test.sh target/appimage/dist/ChipDX-x86_64.AppImage wayland editor
```

Each check runs for 20 seconds in a temporary user directory, without FUSE.
