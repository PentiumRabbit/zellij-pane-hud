#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if ! rustup target list --installed | grep -qx "wasm32-wasip1"; then
    printf '%s\n' "wasm32-wasip1 is not installed. Run: rustup target add wasm32-wasip1" >&2
    exit 1
fi

cargo build -p tree-hud-plugin --target wasm32-wasip1 --release

release_dir="$repo_root/target/wasm32-wasip1/release"
source_wasm=""
for candidate in \
    "$release_dir/tree-hud-plugin.wasm" \
    "$release_dir/tree_hud_plugin.wasm" \
    "$release_dir/libtree_hud_plugin.wasm"
do
    if [[ -f "$candidate" ]]; then
        source_wasm="$candidate"
        break
    fi
done

if [[ -z "$source_wasm" ]]; then
    printf '%s\n' "Could not find the built tree-hud WASM artifact." >&2
    exit 1
fi

mkdir -p "$repo_root/dist"
cp "$source_wasm" "$repo_root/dist/tree-hud.wasm"

printf '\nPlugin: %s\n' "$repo_root/dist/tree-hud.wasm"
printf 'Config: %s\n' "$repo_root/examples/config.kdl"
