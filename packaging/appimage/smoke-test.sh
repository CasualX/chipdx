#!/usr/bin/env bash
set -euo pipefail
image=$(realpath "$1")
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
export HOME="$scratch/home"
export XDG_DATA_HOME="$HOME/.local/share"
export XDG_RUNTIME_DIR="$scratch/runtime"
mkdir -p "$HOME" "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"
export ALSA_CONFIG_PATH="$scratch/alsa.conf"
echo 'pcm.!default { type null }' > "$ALSA_CONFIG_PATH"
export LIBGL_ALWAYS_SOFTWARE=1
# winit 0.30 selects Wayland via these variables; Xvfb supplies DISPLAY.
unset WAYLAND_DISPLAY WAYLAND_SOCKET
export APPIMAGE_EXTRACT_AND_RUN=1
cd "$scratch"
chmod +x "$image"
status=0
timeout 20s xvfb-run -a -s '-screen 0 1024x768x24' "$image" > game.log 2>&1 || status=$?
cat game.log
log_status=0
grep -Eiq 'panicked at|Error (reading|loading|parsing)' game.log || log_status=$?
if [[ $status != 124 || $log_status != 1 ]]; then
    echo "Game failed the startup smoke test (exit $status)." >&2
    exit 1
fi
test -s "$XDG_DATA_HOME/chipdx/chipdx.ini"
test -L "$XDG_DATA_HOME/chipdx/data.paks"
for set in cclp{1..5}; do
    # Symlinks point into a temporary extraction removed when timeout exits.
    test -L "$XDG_DATA_HOME/chipdx/levelsets/$set.paks"
done
test -d "$XDG_DATA_HOME/chipdx/save"
