# Technical research

Research date: 2026-11-14

Target runtime:

- Zellij `0.45.1`
- macOS `x86_64`
- Rust `1.93.0`
- Planned WASM target: `wasm32-wasip1`

## Executive summary

The requested feature is feasible:

- Zellij exposes sessions, tabs and panes through plugin events.
- A plugin can query the foreground command of a terminal pane.
- A plugin can switch to another session and focus a pane in one API call.
- A floating plugin pane can implement the HUD-style search UI.
- A flat fuzzy search and a two-level tab→pane tree are straightforward.

The main constraints are:

- Zellij does not expose a user-defined input mode, so a literal
  `Ctrl+Space` → `Space` sequence requires a launcher with dynamic key
  registration or interception.
- Other live sessions provide structural pane data, but 0.45.1 does not expose
  a reliable live foreground-command feed for every pane in another session.
- Resurrectable sessions have names but no live tab/pane tree until restored.
- The plugin API exposes tabs and panes, not a complete nested split topology.

## Version-specific APIs

### Session and tree discovery

Relevant events:

- `TabUpdate(Vec<TabInfo>)`
- `PaneUpdate(PaneManifest)`
- `SessionUpdate(Vec<SessionInfo>, Vec<(String, Duration)>)`

`SessionInfo` includes tabs, pane manifests, connected-client counts and a
current-session marker. The event is sufficient to render:

```text
session
  tab
    pane
```

Resurrectable sessions are returned as `(name, age)`, so they can only be shown
as leaf nodes.

Versioned source:

- [`event.proto`](https://github.com/zellij-org/zellij/blob/v0.45.1/zellij-utils/src/plugin_api/event.proto)
- [`zellij-tile` event docs](https://zellij.dev/documentation/plugin-api-events.html)

### Pane metadata

`PaneInfo` in 0.45.1 includes:

- stable pane ID;
- plugin vs terminal pane;
- focus, fullscreen, floating and suppressed state;
- title and initial terminal command;
- exited state and exit status;
- pane geometry;
- selectability.

This is enough for row labels, indentation and state icons.

### Current running command

Two APIs are available:

- Subscribe to `CommandChanged(PaneId, Vec<String>, bool, Vec<ClientId>)`.
- Call `get_pane_running_command(PaneId)`.

Both require `ReadApplicationState`. `CommandChanged` reports a new foreground
command when a shell launches a program or returns to the prompt. This supports
live icons such as:

- shell at prompt;
- command running;
- Codex/Claude/editor detected from the executable name.

The best implementation is event-driven:

1. Initialize commands when the popup opens.
2. Update the cache from `CommandChanged`.
3. Avoid polling on every render.

`get_pane_running_command()` and live `CommandChanged` data are reliable for
the current session. For other sessions, structural data is available from
`SessionInfo`; live command state should be treated as best-effort or unknown.

Versioned source:

- [`shim.rs`](https://github.com/zellij-org/zellij/blob/v0.45.1/zellij-tile/src/shim.rs)
- `CommandChanged` in
  [`event.proto`](https://github.com/zellij-org/zellij/blob/v0.45.1/zellij-utils/src/plugin_api/event.proto)

### Direct navigation

Pane focus in the current session:

```rust
focus_terminal_pane(pane_id, should_float_if_hidden, should_be_in_place_if_hidden);
```

Cross-session focus:

```rust
switch_session_with_focus(
    session_name,
    tab_position,
    Some((pane_id, false)),
);
```

The pane is prioritized over the tab position. This makes a one-step jump to
another session's pane feasible.

Required permission: `ChangeApplicationState`.

Versioned documentation:

- [Plugin API commands](https://zellij.dev/documentation/plugin-api-commands.html)

### Search

No Zellij API is needed. The plugin can use a pure-Rust matcher such as
`fuzzy-matcher`, already used by `zellij-pane-picker`.

Search corpus:

- session name;
- tab name and position;
- pane title;
- current command;
- pane ID.

Matching a pane keeps its session and tab ancestors visible. Matching a tab
can keep all its panes visible. This is ordinary tree-filter behavior and has
no external dependency.

### Popup rendering

The plugin can be launched with:

```kdl
LaunchOrFocusPlugin "tree-hud" {
    floating true
    move_to_focused_tab true
}
```

Zellij's `zellij-tile` exposes text/table/nested-list primitives and direct
coordinate rendering. The popup can therefore use the same compact visual
treatment as `zellij-hud`.

The exact mechanism for setting width, height and centered placement still
needs a small implementation spike. It may require plugin-side resize or a
launcher role rather than plain `LaunchOrFocusPlugin` options.

## Shortcut research

### Requested sequence

Desired input:

```text
Ctrl+Space
Space
```

Zellij keybindings support multiple keys in one `bind`, but each listed key is
an independent binding. They do not define a timed sequence.

The first attempted design was a custom mode:

```kdl
keybinds {
    normal {
        bind "Ctrl Space" { SwitchToMode "tree-hud-leader"; }
    }
    tree-hud-leader {
        bind "Space" { LaunchOrFocusPlugin "tree-hud"; }
    }
}
```

This fails validation on Zellij 0.45.1:

```text
Unknown InputMode 'tree-hud-leader'
```

The reproducible fixture is
[`ctrl-space-custom-mode.failed.kdl`](../prototypes/ctrl-space-custom-mode.failed.kdl).

### Feasible strategies

#### A. Single `Ctrl+Space`

Directly opens the finder. This is the simplest, most reliable option and needs
no background launcher.

#### B. Double `Ctrl+Space`

A tiny background launcher receives each `Ctrl+Space` through a global
keybinding, starts a short timer on the first press, and opens the finder if a
second press arrives before expiry. This avoids intercepting normal text keys.

#### C. Exact `Ctrl+Space`, then `Space`

A background launcher can dynamically register a temporary global `Space`
binding after the first chord, or use global key interception for a short
window. Both approaches have tradeoffs:

- dynamic `reconfigure` adds latency and may briefly steal a normal space;
- global interception consumes all keys and would need safe replay for
  non-matching keys;
- a timeout is mandatory to recover from an incomplete sequence.

This remains feasible but should be an optional advanced launcher, not the
core plugin architecture.

Recommendation: make the launcher configurable and start with single
`Ctrl+Space`; implement double `Ctrl+Space` or the literal two-step sequence
after a focused spike.

## Existing plugins

### `zellij-pane-picker`

- Flat pane list with fuzzy filtering and favorites.
- Directly focuses selected panes.
- Good reference for pane search and global shortcut registration.
- Does not provide a session→tab→pane tree.

### `zellij-tree-tabs`

- Persistent vertical tree sidebar.
- Shows tabs and pane lists with status-aware UI.
- Good reference for tree rendering and state.
- It is not a temporary HUD-style popup and does not center on fuzzy search.

### `zellij-leap`

- Search-and-jump interaction inspired by Leap.
- Supports tabs, panes and sessions.
- Good reference for target matching and Enter-to-jump.
- It does not show a persistent tree hierarchy or status icons.

## Configuration model

Proposed plugin configuration:

```kdl
plugins {
    tree-hud location="file:/stable/path/zellij-tree-hud.wasm" {
        scope "all"
        show_sessions "true"
        show_tabs "true"
        show_panes "true"
        include_resurrectable "true"
        show_commands "true"
        icons "nerd"
        icon_style "hud"
        width "72%"
        height "68%"
        follow_current_session "true"
    }
}
```

`show_sessions`, `show_tabs` and `show_panes` control visible levels. Flattening
must remain deterministic when a level is hidden.

## Explicit limitations

- The tree is `session → tab → pane`, not the full nested split geometry.
- Resurrectable sessions cannot expose tabs/panes until they are attached.
- Live running-command icons are guaranteed only for the current session in
  0.45.1.
- The exact two-key sequence needs a launcher separate from the popup plugin.
- The WASM target is not currently installed on this machine.
- The configured Cargo registry is a non-standard USTC sparse source; the build
  phase must decide whether to use it or explicitly use crates.io.
