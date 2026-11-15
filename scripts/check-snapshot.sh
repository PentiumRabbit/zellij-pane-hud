#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if ! command -v zellij-plugin-snapshot >/dev/null 2>&1; then
    printf '%s\n' \
        "zellij-plugin-snapshot is not installed. Run: cargo install zellij-plugin-snapshot --version 0.2.3 --locked" >&2
    exit 1
fi

if [[ ! -s dist/pane-hud.wasm ]]; then
    printf '%s\n' "dist/pane-hud.wasm is missing. Run ./scripts/build-wasm.sh first." >&2
    exit 1
fi

output_dir="$(mktemp -d /tmp/pane-hud-snapshot.XXXXXX)"
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

grep -a -q "FIND" "$smoke"
grep -q "src color renderer" "$smoke_plain"
grep -q "workspace/notes" "$smoke_plain"
grep -q "/work/application" "$smoke_plain"
grep -q "demo/code" "$smoke_plain"
grep -q "codex --full-auto" "$smoke_plain"
grep -q "#1" "$smoke_plain"
grep -a -E -q "38;(2|5);" "$smoke"
grep -a -E -q "1;38;(2|5);" "$smoke"
grep -a -E -q "48;(2|5);" "$smoke"

if grep -q "selected " "$smoke_plain"; then
    printf '%s\n' "Unexpected detail panel heading in smoke snapshot." >&2
    exit 1
fi

grep -q "CX demo/code release tasks" "$template_plain"
grep -q "codex --full-auto" "$template_plain"
grep -a -E -q "1;38;(2|5);" "$template"

if grep -q "selected " "$template_plain"; then
    printf '%s\n' "Unexpected detail panel heading in template snapshot." >&2
    exit 1
fi

printf 'Snapshots: %s\n' "$output_dir"
