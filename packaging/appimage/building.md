# Building the Linux AppImage

The [AppImage user guide](readme.md) covers installation, compatibility and usage.
This document describes packaging, release artifacts and validation.

## GitHub Actions

Run **Build Binaries** (`workflow_dispatch`) on the branch containing the changes.
For a release build, enter the intended GitHub tag in `release_tag`. A build
run from a tag uses that tag when the input is blank.
The `chipdx-appimage-x86_64` artifact contains `ChipDX-x86_64.AppImage` and
`ChipDX-x86_64.AppImage.sha256`. Wait for the Linux startup checks to pass before
replacing both files on the GitHub release.

The image includes `chipplay` and `chipedit` in `usr/bin`, shared assets and five
curated community levelsets. The AppImage runtime launches the root `AppRun`
script. `AppRun` consumes a leading `play` or `edit` subcommand, defaults to
`chipplay`, and passes the remaining arguments to the selected executable.
It sets `CHIPDX_LAUNCH_DIR` before entering the writable user-data directory so
the editor can resolve its level path against the caller's directory.

The AppStream metadata embeds the official project URL. Release builds also
embed the tag, a link to its release page, and the UTC build date. Untagged
builds with an empty `release_tag` carry the project information only.

## Compatibility policy

Builds run inside Ubuntu 20.04, independently of the Actions runner. The ABI
baseline is **glibc 2.31**, x86_64 with baseline x86-64 CPU instructions.
`check-abi.py` audits packaged ELF files and rejects higher glibc requirements.
It also rejects libraries reserved for the host desktop, including the names
recorded in `usr/share/chipdx/host-libraries.txt`.

The host provides glibc, OpenGL/EGL, graphics drivers, Wayland, keyboard/Compose
data and the editor's GTK 3/libnotify stack. On Debian/Ubuntu, the desktop runtime
packages include `libegl1`, the appropriate graphics driver, `libwayland-client0`,
`libwayland-cursor0`, `libwayland-egl1`, `libxkbcommon0`, `xkb-data`,
`libgtk-3-0` (or `libgtk-3-0t64`) and `libnotify4`.
These compatibility targets are independent of distribution security lifecycles.

linuxdeploy bundles the remaining dependencies, including audio libraries.
GTK's full dependency stack (including text rendering and X11 client libraries)
comes from the host; its library names are recorded in the image's
`usr/share/chipdx/host-libraries.txt` and checked by the package audit.
ALSA/PulseAudio connect to the host audio system. Wayland libraries
must stay with the host graphics drivers: bundling Ubuntu 20.04's client library
breaks newer Mesa EGL drivers that require `wl_proxy_marshal_flags` (issue #4).
The game and editor prefer the host xkbcommon. When winit will use X11 and
the host adapter cannot load, `AppRun` adds the private directory containing
`libxkbcommon.so.0`, `libxkbcommon-x11.so.0` and `libxcb-xkb.so.1` to the loader
path. The first two are built together from the pinned upstream source, and
the third comes from the baseline container. The matching core and adapter
avoid crashes from mixing keyboard libraries across versions. Update the
upstream source pin and checksum in `build-x11-fallback.sh` when refreshing
xkbcommon. The bundle includes the notices for both sources.

CI uses software OpenGL and null audio, with Xvfb for X11 and headless Weston
for Wayland. Both the game and editor are checked on Ubuntu, Debian and Fedora.
The X11 jobs use a desktop runtime without the host X11 adapter and verify that
each bundled library loads. A second Ubuntu 22.04 run verifies startup with the
host adapter installed. The Wayland check unsets `DISPLAY`, verifies that each
program submits native Wayland buffers, and checks that the fallback stays out
of the process. Debian 11 jobs use Debian's signed archive repositories because the
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

For a local release build, set `CHIPDX_RELEASE_TAG` when running the build
script, for example `CHIPDX_RELEASE_TAG=v2026-11 bash packaging/appimage/build.sh`.

## Startup checks

With Xvfb, xauth, Weston and the desktop runtime packages installed:

```sh
bash packaging/appimage/smoke-test.sh target/appimage/dist/ChipDX-x86_64.AppImage x11
bash packaging/appimage/smoke-test.sh target/appimage/dist/ChipDX-x86_64.AppImage wayland
bash packaging/appimage/smoke-test.sh target/appimage/dist/ChipDX-x86_64.AppImage x11 editor
bash packaging/appimage/smoke-test.sh target/appimage/dist/ChipDX-x86_64.AppImage wayland editor
```

Use `x11` with the host adapter installed or `x11-fallback` on a test system
without it. Each check runs for 20 seconds in a temporary user directory without
FUSE.
