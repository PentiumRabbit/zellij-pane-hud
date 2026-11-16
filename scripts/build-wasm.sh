#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if ! rustup target list --installed | grep -qx "wasm32-wasip1"; then
    printf '%s\n' "wasm32-wasip1 is not installed. Run: rustup target add wasm32-wasip1" >&2
    exit 1
fi

cargo fetch --locked

zellij_utils_source="$(
    find "${CARGO_HOME:-$HOME/.cargo}/registry/src" \
        -mindepth 2 \
        -maxdepth 2 \
        -type d \
        -name 'zellij-utils-0.45.1' |
        sort |
        head -n 1
)"
if [[ -z "$zellij_utils_source" ]]; then
    printf '%s\n' "Could not locate zellij-utils 0.45.1 in the Cargo registry." >&2
    exit 1
fi

# Zellij 0.45.1 decodes plugin character keys through u8, truncating non-ASCII
# input such as "的" before the plugin can receive it. The host already sends
# the full Unicode scalar value, so patching only the plugin-side decoder is
# sufficient and keeps the WASM protocol compatible.
patch_root="$(mktemp -d "${TMPDIR:-/tmp}/zellij-utils-unicode.XXXXXX")"
lock_backup="$patch_root/Cargo.lock"
cp Cargo.lock "$lock_backup"
restore_build_inputs() {
    cp "$lock_backup" Cargo.lock
    rm -rf "$patch_root"
}
trap restore_build_inputs EXIT
patched_zellij_utils="$patch_root/zellij-utils"
cp -R "$zellij_utils_source" "$patched_zellij_utils"
key_source="$patched_zellij_utils/src/plugin_api/key.rs"
perl -0pi -e \
    's/fn char_index_to_char\(char_index: i32\) -> char \{\n    char_index as u8 as char\n\}/fn char_index_to_char(char_index: i32) -> char {\n    char::from_u32(char_index as u32).unwrap_or(char::REPLACEMENT_CHARACTER)\n}/' \
    "$key_source"
if ! grep -q 'char::from_u32(char_index as u32)' "$key_source"; then
    printf '%s\n' "Failed to patch zellij-utils Unicode key decoding." >&2
    exit 1
fi

cargo build -p pane-switcher-plugin \
    --target wasm32-wasip1 \
    --release \
    --config "patch.crates-io.zellij-utils.path=\"$patched_zellij_utils\""

release_dir="$repo_root/target/wasm32-wasip1/release"
source_wasm=""
for candidate in \
    "$release_dir/pane-switcher-plugin.wasm" \
    "$release_dir/pane_switcher_plugin.wasm" \
    "$release_dir/libpane_switcher_plugin.wasm"
do
    if [[ -f "$candidate" ]]; then
        source_wasm="$candidate"
        break
    fi
done

if [[ -z "$source_wasm" ]]; then
    printf '%s\n' "Could not find the built pane-switcher WASM artifact." >&2
    exit 1
fi

mkdir -p "$repo_root/dist"
cp "$source_wasm" "$repo_root/dist/pane-switcher.wasm"

printf '\nPlugin: %s\n' "$repo_root/dist/pane-switcher.wasm"
printf 'Config: %s\n' "$repo_root/examples/config.kdl"
