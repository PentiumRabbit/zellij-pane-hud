# Zellij Pane HUD Core Design

Status: implemented; updated for the flat pane-template release.

The current product and technical specification is
[`2026-11-15-flat-pane-template-design.md`](2026-11-15-flat-pane-template-design.md).
This document records the crate boundaries and host integration that remained
stable through the row-model refactor.

## Goal

Provide a single floating Zellij plugin launched by user-configured KDL. The
default documented launch path is Zellij's native session leader:

```text
Ctrl+o, then g
```

Zellij 0.45.1 does not expose plugin-defined global shortcuts. The final key
therefore remains in the user's Zellij config and is easy to customize.

## Workspace architecture

```text
crates/pane-hud-core
  deterministic, host-testable configuration, model, search, rows,
  selection, template, style, icons, and renderer

crates/pane-hud-plugin
  Zellij event adapter, command/CWD cache updates, keyboard translation,
  navigation actions, permissions, and render output
```

The core has no Zellij host calls.

## Stable identity

Rows use a composite pane key:

```text
session name + stable Zellij tab ID + terminal pane ID
```

Display position is never identity. Selection is restored by this key after
query or structural refreshes.

## Data flow

```text
Zellij events
  -> adapter snapshots and pane value caches
  -> core World
  -> flat pane rows with template-scoped search results
  -> stable selection and viewport
  -> styled spans
  -> ANSI render lines
```

`SessionUpdate` replaces the live-session snapshot. `TabUpdate` and
`PaneUpdate` update the current session and can arrive independently or out of
order.

`CommandChanged` and `CwdChanged` update the current-session cache. Initial
missing values are queried through the host with bounded retries. There is no
polling loop.

## Row model

- exactly one terminal pane per row;
- no session, tab, dead-session, detail, or fold rows;
- no expansion state;
- Left/Right produce `InputIntent::Ignored`;
- default row format
  `{progress} {icon} {title}  {dir}  {session}/{tab}  {command}  #{pane_id}`;
- text fields in `row_format` define the search corpus automatically;
- `{dir}` is component-safe relative to the focused pane CWD.

## Search

- fuzzy or substring matching;
- UTF-8 character positions returned with scores;
- matches highlighted in bold yellow;
- rows sorted by best field score then stable traversal order;
- empty query preserves current-session-first, tab-position, pane-ID order.

## Rendering

- flat frame with FIND/search line, visible row viewport, footer, and borders;
- no detail panel;
- progress and type icons use independent semantic colors;
- selected row background covers the full width;
- ANSI conversion occurs after Unicode-width truncation and padding;
- render cost is proportional to visible rows, not total rows.

## Navigation

- current session: `focus_terminal_pane`;
- other live session: `switch_session_with_focus` with the terminal pane;
- plugin/non-selectable panes excluded;
- suppressed panes included;
- Enter closes the HUD after dispatching focus;
- Esc closes without navigation.

## Performance contract

- parse the row template once per plugin load;
- rebuild rows only when the query or a template-dependent value changes;
- reuse rows for selection and spinner ticks;
- do not poll command/CWD values;
- schedule spinner timers only while a visible pane is running;
- retain 2,000-pane and 10,000-row regressions.

## Verification

The release verification path is:

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
./scripts/build-wasm.sh
./scripts/check-snapshot.sh
zellij --config examples/config.kdl setup --check
```

A real Zellij smoke test additionally checks launch/render, fuzzy highlight,
full-width selection, Left/Right no-op behavior, Enter navigation, and idle
timer cessation.
