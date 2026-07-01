#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/../.." && pwd)"

cd "$repo_root"

cargo build --release -p chipwasm --target=wasm32-unknown-unknown

mkdir -p code/chiphtml/public
cp target/wasm32-unknown-unknown/release/chipwasm.wasm code/chiphtml/public/chipwasm.wasm
