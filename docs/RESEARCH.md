# Technical research

Research date: 2026-11-15

Target runtime:

- Zellij `0.45.1`
- Rust `1.93.0`
- `wasm32-wasip1`

## Executive summary

The flat pane picker is feasible with the Zellij 0.45.1 plugin APIs:

- `SessionUpdate`, `TabUpdate`, and `PaneUpdate` provide live structural data.
- `CommandChanged` and `get_pane_running_command` provide foreground commands
  for the current session.
- `CwdChanged` and `get_pane_cwd` provide current-session working directories.
- `focus_terminal_pane` focuses current-session panes.
- `switch_session_with_focus` can focus a pane in another live session.
- A floating plugin pane can render a width-bounded ANSI UI.
- `ModeUpdate(ModeInfo)` exposes the active Zellij style and semantic palette.

The main constraints are:

- Zellij does not expose plugin-defined global keybindings.
- Live command/CWD state is not guaranteed for every pane in another session.
- Resurrectable sessions have no live tab/pane tree until attached.
- Zellij does not expose complete nested split geometry through this plugin
  surface.

## Flat pane model

Each row is built from:

```text
session name
tab name + tab position + tab ID
pane title + terminal pane ID
cached CWD
cached command
progress state
search score + match positions
```

Stable identity is the composite key:

```text
session name / stable tab ID / terminal pane ID
```

Display order never identifies a pane.

Normal traversal order is current session first, then tab position, then pane
ID. Query results sort by best field score and retain traversal order for ties.

## Template parsing

`row_format` is parsed once at load. Parsing has three responsibilities:

1. preserve literal text and escaped braces;
2. map known placeholders to typed fields;
3. derive the deduplicated searchable field list.

Generated fields:

- `{progress}`;
- `{icon}`.

Searchable text fields:

- `{title}`;
- `{dir}`;
- `{session}`;
- `{tab}`;
- `{command}`;
- `{pane_id}`.

This avoids maintaining separate search options that can drift from the visible
row layout.

## Relative CWD

The base is the focused current-session pane's CWD. `Path::strip_prefix` gives
component-aware relative paths:

```text
/work/app/src under /work/app -> src
/work/app == /work/app         -> .
/work/application              -> absolute fallback
missing                        -> ?
```

String prefix checks are incorrect because `/work/app` is a string prefix of
`/work/application`.

## Matching and highlighting

The matcher returns:

```rust
struct MatchResult {
    score: MatchScore,
    positions: Vec<usize>,
}
```

`positions` are UTF-8 character indices, not byte offsets. This lets the
renderer split `&str` safely and highlight only matched characters.

Committed IME text may arrive either as ordinary key events or as one
`PastedText` event when the host terminal uses bracketed paste for composition
commits. Both paths append UTF-8 text to the same query model; preedit and
candidate rendering remain terminal-owned.

Zellij 0.45.1 serializes a character key as an `i32`, but its plugin-side
decoder casts the value through `u8`. This corrupts every non-ASCII
`Key::Char`, including a single committed CJK character. The host already sends
the complete Unicode scalar value, so the WASM build applies a
protocol-compatible decoder patch to `zellij-utils`; `PastedText` remains
unaffected because it already carries UTF-8 strings.

Scoring behavior:

- fuzzy mode scans a query as an ordered subsequence;
- substring mode uses direct containment;
- contiguous runs, word-boundary matches, and shorter spans receive bonuses;
- leading gaps and skipped characters receive penalties;
- each visible text field is scored independently;
- the row keeps the best score and positions for every matching field.

No second highlighting search pass is performed.

## Command and CWD data

`CommandChanged(PaneId, Vec<String>, bool, Vec<ClientId>)` reports a foreground
command and whether it is currently in the foreground. The boolean is important
for nested shells:

- `true`: running, even when the executable is `zsh` or `bash`;
- `false`: idle shell;
- exited pane: exited takes precedence;
- missing command: unknown.

`CwdChanged(PaneId, PathBuf, Vec<ClientId>)` updates the CWD without polling.

The plugin performs bounded initial queries after permission/state events:

- command queries only when `{command}`, `{progress}`, or `{icon}` is active;
- CWD queries only when `{dir}` is active;
- at most two failed attempts per pane and field per plugin lifetime.

Current-session values are reliable. Other-session rows remain navigable even
when these values are unavailable.

## Navigation APIs

Current session:

```rust
focus_terminal_pane(pane_id, true, false);
```

Another live session:

```rust
switch_session_with_focus(
    session_name,
    Some(tab_position),
    Some((pane_id, false)),
);
```

Both require the plugin's `ChangeApplicationState` permission.

Plugin panes and non-selectable UI panes are filtered in the adapter.
Suppressed terminal panes remain selectable.

## Shortcut research

The requested launch sequence involved `Ctrl+Space`. Zellij 0.45.1 keybinding
limitations:

- listed bindings are independent, not timed sequences;
- custom input modes are rejected as `Unknown InputMode`;
- plugin-defined global shortcut registration is unavailable;
- dynamic global bindings or interception would add a background launcher and
  complex replay/timeout behavior.

The local environment already uses `Ctrl+Space` and `Ctrl+g`, so neither can be
claimed safely.

Implemented release path:

```kdl
keybinds {
    shared_except "locked" {
        bind "Ctrl Shift e" {
            SwitchToMode "normal"
            LaunchOrFocusPlugin "pane-switcher" {
                floating true
                move_to_focused_tab true
            }
        }
    }
    session {
        bind "g" {
            LaunchOrFocusPlugin "pane-switcher" {
                floating true
                move_to_focused_tab true
            }
            SwitchToMode "Normal"
        }
    }
}
```

This reuses Zellij's native session leader:

```text
Ctrl+o, then g
```

The final key stays visible and configurable in ordinary Zellij KDL. No
background launcher, key interception, or normal-space hijacking is required.

## Plugin update and cache research

GitHub's `releases/latest/download` path resolves to the newest non-prerelease
asset at fetch time. Zellij also caches loaded plugin artifacts, so a stable
`latest` URL can otherwise continue using an older WASM file.

The preferred startup strategy is an external launcher check before a new
Zellij server starts. It queries GitHub's latest releases, reads their asset
digests, and removes numeric files in Zellij's plugin download cache that do not
match any current configured plugin. The next plugin launch downloads the
latest HUD asset and subsequent launches reuse it.

`LaunchOrFocusPlugin` also supports `skip_plugin_cache true`. It bypasses memory
and on-disk plugin caches for a newly launched pane, providing a native fallback
when no startup checker exists. Its tradeoff is remote resolution and
compilation on every new launch.

Neither strategy hot-swaps a live plugin instance. Users close and reopen the
pane to update. Versioned release URLs remain available when deterministic
startup matters more than upgrades.

## Rendering research

ANSI escape bytes cannot be used for terminal width calculations. The renderer
therefore:

1. evaluates template fields into styled spans;
2. splits matched text into bold-yellow and normal spans;
3. truncates by Unicode display width;
4. pads to the exact row width;
5. applies selected-background state to every visible span, including padding;
6. converts spans to ANSI last.

This keeps:

- CJK and other wide characters width-safe;
- icon/match colors visible on selected rows;
- the selected background full-width;
- footer/search/border lines aligned.

Zellij's `Text` and `Table` components expose theme-relative emphasis levels,
but only a small fixed set of semantic foregrounds. The HUD needs independent
progress, command-category, field, match, and selection colors. Text fields use
cyan, blue, magenta, orange, green, and dark gray roles, while matched
characters use bold yellow. It therefore uses `ModeInfo.style.colors` to build
a semantic ANSI palette while retaining Zellij as the source of truth for the
active theme.

## Performance findings

Expected usage is tens to a few hundred panes. The implementation keeps work
proportional to changed data and visible output:

- template parsed once;
- searchable fields derived once with the template;
- cached rows reused by render, selection, and spinner ticks;
- only visible rows styled and converted to ANSI;
- selected viewport computed by arithmetic rather than scanning the prefix;
- command/CWD updates invalidate only when the template depends on them;
- no process polling;
- timer stops when no visible pane is running.

Regressions exercise:

- 2,000-pane Finder updates;
- 10,000-row selected viewport rendering;
- repeated render/selection without row rebuilds;
- duplicate structural events without render requests;
- idle shell without spinner timers;
- bounded failed queries.

## Versioned source references

- `SessionUpdate`, `TabUpdate`, `PaneUpdate`, `CommandChanged`, `CwdChanged`:
  Zellij `zellij-utils/src/data.rs` and
  `zellij-utils/src/plugin_api/event.proto` at `v0.45.1`.
- Plugin queries and navigation commands:
  Zellij `zellij-tile/src/shim.rs` at `v0.45.1`.
- Host event conversion:
  `zellij-utils/src/plugin_api/event.rs` at `v0.45.1`.
- Snapshot acceptance:
  `zellij-plugin-snapshot 0.2.3` with `zellij-tile 0.45.1`.

## Remaining limitations

- No plugin-defined global shortcut.
- No guaranteed live command/CWD details for other sessions.
- No resurrectable session rows because they have no live pane target.
- No nested split topology visualization.
- An open plugin instance is not hot-swapped when a newer release appears.
- `LaunchOrFocusPlugin` has no width/height fields in 0.45.1, but plugins can
  apply their own dimensions after permission using
  `change_floating_panes_coordinates`.
