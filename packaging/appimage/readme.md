# Linux AppImage

Run **Build Binaries** from GitHub Actions (`workflow_dispatch`). The
`chipdx-appimage-x86_64` artifact contains `ChipDX-x86_64.AppImage` and its SHA256
checksum, ready to attach to a release. The image contains the game, assets and
five curated community levelsets. The level editor is not included.

## Compatibility

Builds run inside Ubuntu 20.04, even when the Actions runner is newer. The target
is x86_64 Linux with **glibc 2.31 or later**, with baseline x86-64 CPU instructions.
This includes Ubuntu 20.04+, Debian 11+, and comparable distributions; it does
not include musl-based systems such as Alpine. An ELF symbol audit rejects
bundled libraries that would raise the glibc requirement.

linuxdeploy bundles library dependencies, including those dynamically loaded by
winit and miniaudio. The host supplies glibc, graphics drivers, an X11 or Wayland
desktop, and OpenGL 3.3. ALSA/PulseAudio connect to the host audio system. This
does not guarantee every driver/compositor combination. CI exercises X11 startup
using software OpenGL on Ubuntu 20.04, 22.04, 24.04, and Debian 12, with null audio.
Linux builds prefer EGL with an sRGB surface, falling back to GLX. This avoids
requiring sRGB GLX configurations on GPU-free Xvfb runners.
Native Wayland, real sound output and gamepads still need manual testing.

The linuxdeploy release and the statically linked AppImage runtime are pinned
and checksum-verified. linuxdeploy includes the AppImage output tool. It runs
using extract-and-run, so building needs neither FUSE nor a privileged container.

## Running

```sh
chmod +x ChipDX-x86_64.AppImage
./ChipDX-x86_64.AppImage
```

If FUSE is unavailable, use:

```sh
./ChipDX-x86_64.AppImage --appimage-extract-and-run
```

Configuration, saves and additional levelsets live under
`${XDG_DATA_HOME:-$HOME/.local/share}/chipdx/`. Edit `chipdx.ini` there and put
extra `.dat` files in its `levelsets/` directory. Keep the image while running:
bundled assets are linked from the mounted/extracted image. Upgrading the image
preserves configuration and saves, and refreshes links to bundled levelsets.

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
