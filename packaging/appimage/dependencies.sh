#!/usr/bin/env bash
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive
apt-get update -o Acquire::Languages=none
apt-get install -y --no-install-recommends \
    ca-certificates curl build-essential cmake pkg-config python3 file \
    libudev-dev libx11-dev libx11-xcb-dev libxcursor-dev libxrandr-dev \
    libxi-dev libxinerama-dev libxrender-dev libxfixes-dev \
    libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev \
    libasound2-dev libpulse-dev libgl1-mesa-dev libegl1-mesa-dev
