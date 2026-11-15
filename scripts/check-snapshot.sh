#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if ! command -v zellij-plugin-snapshot >/dev/null 2>&1; then
    printf '%s\n' \
        "zellij-plugin-snapshot is not installed. Run: cargo install zellij-plugin-snapshot --version 0.2.3 --locked" >&2
    exit 1
fi

if [[ ! -s dist/tree-hud.wasm ]]; then
    printf '%s\n' "dist/tree-hud.wasm is missing. Run ./scripts/build-wasm.sh first." >&2
    exit 1
fi

output_dir="$(mktemp -d /tmp/tree-hud-snapshot.XXXXXX)"
for fixture in smoke template; do
    zellij-plugin-snapshot "tests/snapshot/${fixture}.yaml" --out "$output_dir"
done

smoke="$output_dir/smoke.ansi.txt"
template="$output_dir/template.ansi.txt"
smoke_plain="$output_dir/smoke.txt"
template_plain="$output_dir/template.txt"

test -s "$smoke"
test -s "$template"
perl -pe 's/\e\[[0-9;]*m//g' "$smoke" >"$smoke_plain"
perl -pe 's/\e\[[0-9;]*m//g' "$template" >"$template_plain"

rg -a -q "FIND" "$smoke"
rg -q "src color renderer" "$smoke_plain"
rg -q "workspace/notes" "$smoke_plain"
rg -q "/work/application" "$smoke_plain"
rg -a -q "38;5;(51|196|245|75|201|208)" "$smoke"
rg -a -q "1;38;5;226" "$smoke"
rg -a -q "48;5;24" "$smoke"

if rg -q "selected " "$smoke_plain"; then
    printf '%s\n' "Unexpected detail panel heading in smoke snapshot." >&2
    exit 1
fi

rg -q "CX demo/code release tasks" "$template_plain"
rg -q "codex --full-auto" "$template_plain"
rg -a -q "1;38;5;226" "$template"

if rg -q "selected " "$template_plain"; then
    printf '%s\n' "Unexpected detail panel heading in template snapshot." >&2
    exit 1
fi

printf 'Snapshots: %s\n' "$output_dir"
