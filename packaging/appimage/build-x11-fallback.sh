#!/usr/bin/env bash
set -euo pipefail

work=$1
appdir=$2
tools="$work/tools"
scratch=$(mktemp -d "$work/x11-build.XXXXXX")
trap 'rm -rf "$scratch"' EXIT

fetch() {
    local file="$tools/$1" url=$2 checksum=$3
    if [[ ! -f "$file" ]]; then
        curl -fL --retry 3 "$url" -o "$file.download"
        mv "$file.download" "$file"
    fi
    printf '%s  %s\n' "$checksum" "$file" | sha256sum -c -
}

xkb_version=1.13.2
xkb_archive="libxkbcommon-$xkb_version.tar.gz"
meson_wheel=meson-1.7.2-py3-none-any.whl
bison_archive=bison-3.8.2.tar.xz
fetch "$xkb_archive" \
    "https://codeload.github.com/xkbcommon/libxkbcommon/tar.gz/refs/tags/xkbcommon-$xkb_version" \
    acc4d5f7c3cbba5f9f8d08d8bdbeede84ecede46792f47929aa9321873385528
fetch "$meson_wheel" \
    "https://files.pythonhosted.org/packages/e5/2b/46bda4ef5a7ae4135dbfe27fc0368c44e5a349a897a54fdf2cedb8dcb66e/$meson_wheel" \
    82c6818dc81743c96de3a458f06175776ebfde4081195ea31ea6971838f25e38
fetch "$bison_archive" "https://ftp.gnu.org/gnu/bison/$bison_archive" \
    9bba0214ccf7f1079c5d59210045227bcf619519840ebfa80cd3849cff5a5bf2

# Focal's Meson and Bison are too old for this xkbcommon release.
python3 -m zipfile -e "$tools/$meson_wheel" "$scratch/meson"
mkdir "$scratch/bison-src" "$scratch/xkb-src"
tar -xf "$tools/$bison_archive" -C "$scratch/bison-src" --strip-components=1
tar -xf "$tools/$xkb_archive" -C "$scratch/xkb-src" --strip-components=1
(
    cd "$scratch/bison-src" &&
    ./configure --prefix="$scratch/bison" --disable-nls --without-libtextstyle &&
    make -j"$CARGO_BUILD_JOBS" &&
    make install
) > "$work/bison-build.log" 2>&1 || { tail -80 "$work/bison-build.log" >&2; exit 1; }
export PATH="$scratch/bison/bin:$PATH"

meson=(env PYTHONPATH="$scratch/meson" python3 -m mesonbuild.mesonmain)
"${meson[@]}" setup "$scratch/xkb-build" "$scratch/xkb-src" \
    --prefix=/usr --buildtype=release --wrap-mode=nodownload \
    -Denable-tools=false -Denable-wayland=false -Denable-xkbregistry=false \
    -Denable-bash-completion=false
"${meson[@]}" compile -C "$scratch/xkb-build" -j "$CARGO_BUILD_JOBS" xkbcommon-x11

fallback="$appdir/usr/lib/chipdx-x11"
notices="$appdir/usr/share/chipdx/licenses"
install -d "$fallback" "$notices" "$appdir/usr/libexec/chipdx"
install -m644 "$scratch/xkb-build/libxkbcommon.so.0" "$fallback/libxkbcommon.so.0"
install -m644 "$scratch/xkb-build/libxkbcommon-x11.so.0" "$fallback/libxkbcommon-x11.so.0"
install -m644 "$(pkg-config --variable=libdir xcb-xkb)/libxcb-xkb.so.1" "$fallback/libxcb-xkb.so.1"
strip --strip-unneeded "$fallback/libxkbcommon.so.0" "$fallback/libxkbcommon-x11.so.0"
install -m644 "$scratch/xkb-src/LICENSE" "$notices/libxkbcommon.txt"
install -m644 /usr/share/doc/libxcb-xkb1/copyright "$notices/libxcb-xkb1.txt"
cc -O2 -Wall -Wextra -Werror packaging/appimage/probe-xkbcommon-x11.c \
    -o "$appdir/usr/libexec/chipdx/probe-xkbcommon-x11" -ldl
