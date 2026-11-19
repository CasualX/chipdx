#!/usr/bin/env bash
set -euo pipefail
image=$(realpath "$1")
backend=${2:-x11}
program=${3:-game}
launch_args=()
case "$program" in
    game) ;;
    editor) launch_args=(edit) ;;
    *) echo "Unknown program: $program" >&2; exit 1 ;;
esac
case "$backend" in
    x11|x11-fallback|wayland) ;;
    *) echo "Unknown display backend: $backend" >&2; exit 1 ;;
esac
scratch=$(mktemp -d)
compositor=
cleanup() {
    if [[ -n "$compositor" ]]; then
        kill "$compositor" 2>/dev/null || true
        wait "$compositor" 2>/dev/null || true
    fi
    rm -rf "$scratch"
}
trap cleanup EXIT
export HOME="$scratch/home"
export XDG_DATA_HOME="$HOME/.local/share"
export XDG_RUNTIME_DIR="$scratch/runtime"
mkdir -p "$HOME" "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"
export ALSA_CONFIG_PATH="$scratch/alsa.conf"
echo 'pcm.!default { type null }' > "$ALSA_CONFIG_PATH"
export LIBGL_ALWAYS_SOFTWARE=1
# Select the backend independently of the desktop running the test.
unset DISPLAY WAYLAND_DISPLAY WAYLAND_SOCKET
export APPIMAGE_EXTRACT_AND_RUN=1
# Resolve driver symbols immediately: otherwise the old-Wayland regression can
# surface as a loader error instead of glutin's misleading GLX fallback panic.
export LD_BIND_NOW=1
cd "$scratch"
chmod +x "$image"
run_image=(env LD_DEBUG=libs LD_DEBUG_OUTPUT="$scratch/loader" "$image" "${launch_args[@]}")
status=0
if [[ "$backend" == wayland ]]; then
    export WAYLAND_DISPLAY=wayland-chipdx-test
    weston_args=(--backend=headless-backend.so --use-pixman)
    if [[ $(weston --help 2>&1) == *--renderer* ]]; then
        weston_args=(--backend=headless --renderer=pixman)
    fi
    weston "${weston_args[@]}" --no-config \
        --socket="$WAYLAND_DISPLAY" --idle-time=0 > weston.log 2>&1 &
    compositor=$!
    for ((attempt=0; attempt<100; attempt++)); do
        [[ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]] && break
        kill -0 "$compositor" 2>/dev/null || break
        sleep 0.1
    done
    if [[ ! -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]]; then
        cat weston.log >&2
        echo 'Headless Wayland compositor failed to start.' >&2
        exit 1
    fi
    WAYLAND_DEBUG=1 timeout 20s "${run_image[@]}" > game.log 2>&1 || status=$?
else
    timeout 20s xvfb-run -a -s '-screen 0 1024x768x24' "${run_image[@]}" > game.log 2>&1 || status=$?
fi
log_status=0
grep -Eiq 'panicked at|symbol lookup error|unrecognized keysym|Error (reading|loading|parsing)' game.log || log_status=$?
if [[ $status != 124 || $log_status != 1 ]]; then
    tail -80 game.log >&2
    echo "Game failed the startup smoke test (exit $status)." >&2
    exit 1
fi
loader_logs=(loader.*)
if [[ ! -f ${loader_logs[0]} ]]; then
    echo 'Missing dynamic loader logs.' >&2
    exit 1
fi
fallback_init='calling init: .*/usr/lib/chipdx-x11/'
if [[ "$backend" == x11-fallback ]]; then
    for library in libxkbcommon.so.0 libxkbcommon-x11.so.0 libxcb-xkb.so.1; do
        if ! grep -Eq "$fallback_init${library//./\\.}$" "${loader_logs[@]}"; then
            echo "X11 fallback did not load $library." >&2
            exit 1
        fi
    done
elif grep -q "$fallback_init" "${loader_logs[@]}"; then
    echo "Unexpected use of the packaged X11 fallback on $backend." >&2
    exit 1
fi
if [[ "$backend" == wayland ]]; then
    # A live process alone is insufficient: verify native Wayland buffers were
    # attached and committed, rather than silently falling back to Xwayland.
    # libwayland versions use either interface@id or interface#id in debug logs.
    if ! grep -Eq 'wl_surface[@#][0-9]+\.attach\(wl_buffer[@#][0-9]+' game.log ||
       ! grep -Eq 'wl_surface[@#][0-9]+\.commit\(' game.log; then
        tail -80 game.log >&2
        echo 'Game did not submit native Wayland buffers.' >&2
        exit 1
    fi
else
    cat game.log
fi
test -s "$XDG_DATA_HOME/chipdx/chipdx.ini"
test -L "$XDG_DATA_HOME/chipdx/data.paks"
for set in cclp{1..5}; do
    # Symlinks point into a temporary extraction removed when timeout exits.
    test -L "$XDG_DATA_HOME/chipdx/levelsets/$set.paks"
done
test -d "$XDG_DATA_HOME/chipdx/save"
echo "AppImage $program $backend startup smoke test passed."
