#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."

if [[ $(uname -m) != x86_64 || $(getconf GNU_LIBC_VERSION) != 'glibc 2.31' ]]; then
    echo 'Build inside Ubuntu 20.04 (x86_64, glibc 2.31); see packaging/appimage/readme.md.' >&2
    exit 1
fi

# Isolate the baseline build from artifacts compiled on a newer host.
export CARGO_TARGET_DIR="$PWD/target/appimage-build"
export RUSTFLAGS='-C target-cpu=x86-64'
export CFLAGS='-march=x86-64 -mtune=generic'
export CXXFLAGS="$CFLAGS"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
work="$PWD/target/appimage"
appdir="$work/ChipDX.AppDir"
mkdir -p "$work/tools" "$work/dist"
if [[ -e "$appdir" ]]; then
    echo "Remove $appdir before rebuilding." >&2
    exit 1
fi

tool="$work/tools/linuxdeploy-x86_64.AppImage"
if [[ ! -f "$tool" ]]; then
    curl -fL --retry 3 \
        https://github.com/linuxdeploy/linuxdeploy/releases/download/1-alpha-20251107-1/linuxdeploy-x86_64.AppImage \
        -o "$tool.download"
    mv "$tool.download" "$tool"
fi
# SHA256 is pinned alongside the release, so a changed download fails closed.
echo 'c20cd71e3a4e3b80c3483cef793cda3f4e990aca14014d23c544ca3ce1270b4d  '"$tool" | sha256sum -c -
chmod +x "$tool"

runtime="$work/tools/runtime-x86_64"
if [[ ! -f "$runtime" ]]; then
    curl -fL --retry 3 \
        https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-x86_64 \
        -o "$runtime.download"
    mv "$runtime.download" "$runtime"
fi
echo '2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d  '"$runtime" | sha256sum -c -
export LDAI_RUNTIME_FILE="$runtime"

# This repository does not track Cargo.lock (same resolution policy as Windows).
cargo build --release -p chipplay -p chipbundle
assets="$appdir/usr/share/chipdx"
mkdir -p "$assets" "$appdir/usr/bin"
"$CARGO_TARGET_DIR/release/chipbundle" "$assets"
cp chipdx.ini license.md "$assets/"
install -m755 "$CARGO_TARGET_DIR/release/chipplay" "$appdir/usr/bin/chipplay"
install -m755 packaging/appimage/AppRun "$appdir/AppRun"
cp code/android/icon.svg "$appdir/chipdx.svg"

# winit and miniaudio dlopen these: ldd alone does not discover them.
# Leave glibc, libGL/libEGL and graphics drivers on the host.
libraries=(
    libX11.so.6 libX11-xcb.so.1 libXcursor.so.1 libXrandr.so.2
    libXi.so.6 libXinerama.so.1 libXrender.so.1 libXfixes.so.3
    libwayland-client.so.0 libwayland-cursor.so.0 libwayland-egl.so.1
    libxkbcommon.so.0 libxkbcommon-x11.so.0 libasound.so.2
    libpulse.so.0 libudev.so.1
)
extra=()
for library in "${libraries[@]}"; do
    path=$(ldconfig -p | awk -v name="$library" '$1 == name && /x86-64/ && !found++ {print $NF}')
    [[ -n "$path" ]] || { echo "Missing library: $library" >&2; exit 1; }
    extra+=(--library "$path")
done

export APPIMAGE_EXTRACT_AND_RUN=1
"$tool" --appdir "$appdir" "${extra[@]}" \
    --desktop-file packaging/appimage/chipdx.desktop --icon-file "$appdir/chipdx.svg"
python3 packaging/appimage/check-abi.py "$appdir"

export ARCH=x86_64
export OUTPUT="$work/dist/ChipDX-x86_64.AppImage"
"$tool" --appdir "$appdir" --output appimage
python3 packaging/appimage/check-abi.py "$OUTPUT"
cd "$work/dist"
sha256sum ChipDX-x86_64.AppImage > ChipDX-x86_64.AppImage.sha256
