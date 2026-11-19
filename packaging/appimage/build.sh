#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."

if [[ $(uname -m) != x86_64 || $(getconf GNU_LIBC_VERSION) != 'glibc 2.31' ]]; then
    echo 'Build inside Ubuntu 20.04 (x86_64, glibc 2.31); see packaging/appimage/building.md.' >&2
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
cargo build --release -p chipplay -p chipedit -p chipbundle
assets="$appdir/usr/share/chipdx"
mkdir -p "$assets" "$appdir/usr/bin"
"$CARGO_TARGET_DIR/release/chipbundle" "$assets"
cp chipdx.ini license.md "$assets/"
install -m755 "$CARGO_TARGET_DIR/release/chipplay" "$appdir/usr/bin/chipplay"
install -m755 "$CARGO_TARGET_DIR/release/chipedit" "$appdir/usr/bin/chipedit"
install -m755 packaging/appimage/AppRun "$appdir/AppRun"
cp code/android/icon.svg "$appdir/chipdx.svg"

app_id=net.casualhacks.chipdx
metainfo="$appdir/usr/share/metainfo/$app_id.appdata.xml"
install -Dm644 "packaging/appimage/$app_id.appdata.xml" "$metainfo"
release_tag=${CHIPDX_RELEASE_TAG:-}
if [[ -z "$release_tag" && ${GITHUB_REF_TYPE:-} == tag ]]; then
    release_tag=${GITHUB_REF_NAME:-}
fi
if [[ -n "$release_tag" ]]; then
    python3 - "$metainfo" "$release_tag" "$(date -u +%F)" <<'PY'
import sys
import xml.etree.ElementTree as ET
from urllib.parse import quote

path, tag, date = sys.argv[1:]
tree = ET.parse(path)
component = tree.getroot()
homepage = component.findtext("url[@type='homepage']")
releases = ET.SubElement(component, "releases")
release = ET.SubElement(releases, "release", version=tag, date=date)
ET.SubElement(release, "url", type="details").text = f"{homepage}/releases/tag/{quote(tag, safe='')}"
tree.write(path, encoding="utf-8", xml_declaration=True)
PY
    export VERSION="$release_tag"
fi

# winit and miniaudio dlopen these: ldd alone does not discover them.
# Leave glibc, graphics drivers, Wayland and the default keyboard stack on the host.
# New host EGL drivers can require Wayland symbols absent from Ubuntu 20.04;
# old xkbcommon also cannot parse newer host Compose tables (issue #4).
libraries=(
    libX11.so.6 libX11-xcb.so.1 libXcursor.so.1 libXrandr.so.2
    libXi.so.6 libXinerama.so.1 libXrender.so.1 libXfixes.so.3
    libasound.so.2
    libpulse.so.0 libudev.so.1
)
# Apply exclusions to both passes, including dependencies found transitively.
# The editor uses the host GTK/GLib stack for native file dialogs. Bundling an
# older GLib would also break newer host GTK modules and desktop integration.
host_patterns=(
    'libwayland-*.so*' 'libxkbcommon*.so*' 'libxcb-xkb.so*'
    'libgtk-3.so*' 'libgdk-3.so*'
    'libglib-2.0.so*' 'libgobject-2.0.so*'
    'libgio-2.0.so*' 'libgmodule-2.0.so*'
    'libgthread-2.0.so*' 'libnotify.so*'
)
# linuxdeploy traverses excluded libraries too. Exclude GTK's entire dependency
# graph so old Pango, Cairo, etc. cannot shadow their newer host counterparts.
gtk_dependencies=$(ldd "$(pkg-config --variable=libdir gtk+-3.0)/libgtk-3.so.0" \
    "$(pkg-config --variable=libdir libnotify)/libnotify.so.4")
if [[ "$gtk_dependencies" == *'not found'* ]]; then
    echo "$gtk_dependencies" >&2
    exit 1
fi
mapfile -t gtk_libraries < <(awk '$2 == "=>" {print $1}' <<< "$gtk_dependencies" | sort -u)
host_patterns+=("${gtk_libraries[@]}")
printf '%s\n' "${host_patterns[@]}" > "$assets/host-libraries.txt"
host_libraries=()
for pattern in "${host_patterns[@]}"; do
    host_libraries+=(--exclude-library "$pattern")
done
extra=()
for library in "${libraries[@]}"; do
    host_library=false
    for pattern in "${host_patterns[@]}"; do
        if [[ "$library" == $pattern ]]; then host_library=true; break; fi
    done
    "$host_library" && continue
    path=$(ldconfig -p | awk -v name="$library" '$1 == name && /x86-64/ && !found++ {print $NF}')
    [[ -n "$path" ]] || { echo "Missing library: $library" >&2; exit 1; }
    extra+=(--library "$path")
done

export APPIMAGE_EXTRACT_AND_RUN=1
"$tool" --appdir "$appdir" "${host_libraries[@]}" "${extra[@]}" \
    --desktop-file "packaging/appimage/$app_id.desktop" --icon-file "$appdir/chipdx.svg"
bash packaging/appimage/build-x11-fallback.sh "$work" "$appdir"
python3 packaging/appimage/check-abi.py "$appdir"

export ARCH=x86_64
export OUTPUT="$work/dist/ChipDX-x86_64.AppImage"
export AS_VALIDATE_NONET=1
"$tool" --appdir "$appdir" "${host_libraries[@]}" --output appimage
python3 packaging/appimage/check-abi.py "$OUTPUT"
cd "$work/dist"
sha256sum ChipDX-x86_64.AppImage > ChipDX-x86_64.AppImage.sha256
