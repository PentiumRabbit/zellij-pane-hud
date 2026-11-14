# Design proposal

Status: draft for review. No implementation has started.

## Product definition

`zellij-tree-hud` is a temporary, floating search UI for jumping to sessions,
tabs and panes. It combines:

- the compact visual language of `zellij-hud`;
- the search-first interaction of Neovim pickers;
- a session/tab/pane tree;
- live pane-state icons and selected-pane details;
- direct cross-session pane focus.

It is not a permanent sidebar and does not replace `zellij-hud`.

## User flow

1. The user invokes the finder from a configurable shortcut.
2. A centered floating pane opens with focus in the search field.
3. The tree shows live sessions first, with expandable tabs and panes.
4. Typing filters the tree and automatically expands matching branches.
5. Up/Down selects a session, tab or pane.
6. Enter jumps to the selected target and closes the finder.
7. Esc closes the finder without navigation.

Proposed visual:

```text
╭─ FIND ─────────────────────────────────────────────╮
│ app                                            Esc │
│ ────────────────────────────────────────────────── │
│ ▾󰒍 work                                   current  │
│   ▾󰓩 code                                  2 panes │
│     ● 󰈙 nvim src/main.rs                     nvim  │
│     >_ zsh                                   shell │
│   ▸󰓩 tests                                 1 pane  │
│ ▸󰒍 infra                                             │
│  󰒓 old-session                resurrectable 2d ago  │
│                                                     │
│ ── selected ─────────────────────────────────────── │
│ status   running                                    │
│ cwd      ~/Work/GitSource/ConfigFiles               │
│ command  cargo test --all                           │
│ pane     work / code / 3                            │
│                                                     │
│ ↑↓ move  ←→ fold  Enter jump  Esc close             │
╰─────────────────────────────────────────────────────╯
```

The final glyphs should be centralized so plain-text and Nerd Font modes remain
possible.

The detail block is a selected-node preview, not a permanent layout panel.
When the popup is wide enough it can move to the right side; narrow terminals
keep it below the tree.

## Visible levels

All three levels are configurable:

- `show_sessions=true`: roots are sessions.
- `show_sessions=false`: roots are tabs from the current/all live sessions.
- `show_tabs=true`: panes are nested under tabs.
- `show_tabs=false`: panes are direct children of their session.
- `show_panes=false`: only session/tab navigation is available.

The default is all three levels enabled.

When session names collide across current and dead sessions, the UI must keep
their types distinct.

## Search behavior

Search is case-insensitive fuzzy matching across:

- session name;
- tab name;
- pane title;
- current working directory;
- current command;
- pane ID.

Tree filtering rules:

- Matching a pane preserves its ancestors.
- Matching a tab shows the tab and its matching/descendant panes.
- Matching a session shows its tree.
- Empty query shows the configured normal tree.
- Results score panes above tabs, and tabs above sessions when scores tie.
- The currently focused pane is selected on initial open when available.

The search field must support:

- printable ASCII and UTF-8 input;
- Backspace;
- `Ctrl+w` word deletion;
- `Ctrl+u` clear query;
- Up/Down and `Ctrl+p`/`Ctrl+n` navigation;
- Left/Right to collapse/expand branches;
- Enter to jump;
- Esc to close.

## State icons

Default Nerd Font set:

| State | Suggested glyph | Meaning |
|---|---|---|
| focused | `●` | Current focused terminal pane |
| shell | `>_` | Shell is foreground |
| command | `●` or spinner | Non-shell command is running |
| codex | `❉` | Running command starts with `codex` |
| claude | `✳` | Running command starts with `claude` |
| editor | `󰈙` | nvim/vim/helix detected |
| floating | `󰹙` | Floating pane |
| fullscreen | `󰊓` | Fullscreen pane |
| stacked | `󰐕` | Suppressed/stacked pane |
| exited | `󰅖` | Exited pane |
| unknown | `?` | Status unavailable |

The icon state is derived in this order:

1. plugin/terminal type;
2. exited;
3. focused;
4. running command category;
5. floating/fullscreen/suppressed modifiers;
6. placeholder.

Running commands should be updated from `CommandChanged`; the plugin should not
spawn a process poller.

## Selected pane details

The details area shows full, unclipped information for the selected terminal
pane:

- progress state: running, idle shell, exited or unknown;
- current working directory;
- full foreground command and arguments;
- session, tab and pane identity;
- floating/fullscreen/stacked modifiers when relevant.

The short pane row keeps only compact information so long commands do not
destroy the tree alignment. The detail panel provides the complete values.

Layout behavior:

- `details=auto`: right preview at wide widths, bottom block at narrow widths;
- `details=bottom`: always draw below the visible tree;
- `details=right`: always reserve a side preview;
- `details=off`: hide the panel.

Current-session CWD uses `CwdChanged` and `get_pane_cwd()`. Current-session
commands use `CommandChanged` and `get_pane_running_command()`. Other live
sessions may show `unknown` for these values because 0.45.1 does not guarantee
live process details for their panes.

Switching to another session with `switch_session_with_focus` is still
available even when its CWD or command is unknown.

## Navigation semantics

Enter behavior by node:

- Session: switch to that session.
- Tab: switch to the tab in its session.
- Pane: switch to its session/tab and focus the terminal pane.
- Resurrectable session: attach/restore it. No pane target is available yet.

The current session uses `focus_terminal_pane`. Any other live session uses
`switch_session_with_focus`.

Suppressed panes should be included by default and unsuppressed on navigation.
Plugin panes are excluded from selectable targets.

## Configuration

Proposed KDL:

```kdl
plugins {
    tree-hud location="file:/stable/path/zellij-tree-hud.wasm" {
        scope "all"                    // current | all
        show_sessions "true"
        show_tabs "true"
        show_panes "true"
        include_resurrectable "true"
        show_commands "true"
        show_cwd "true"
        details "auto"                 // auto | bottom | right | off
        detail_status "true"
        detail_cwd "true"
        detail_command "true"
        show_pane_ids "false"
        follow_current_session "true"
        sort "session-tab-pane"
        icons "nerd"                   // nerd | plain
        density "compact"              // compact | comfortable
        width "72%"
        height "68%"
        search_mode "fuzzy"            // fuzzy | substring
        spinner_fps "4"
    }
}
```

Unknown keys are ignored. Unknown enum values fall back to defaults and should
not crash the plugin.

## Shortcut strategy

The target shortcut is `Ctrl+Space`, then `Space`, with a configurable timeout.

Two components are required:

1. A small background launcher that recognizes the sequence.
2. The floating finder plugin that owns search and navigation.

Recommended development order:

1. Build the finder with direct `Ctrl+Space` launch.
2. Add a double-`Ctrl+Space` launcher as a stable alternative.
3. Spike the exact `Ctrl+Space` → `Space` mode and keep it behind a config flag.

The exact sequence must not consume normal spaces outside its short pending
window.

## Architecture

Proposed modules:

```text
src/
  main.rs        plugin registration and event loop
  config.rs      plugin configuration parsing
  model.rs       Session/Tab/Pane tree and stable IDs
  events.rs      Zellij event/action adapter
  search.rs      fuzzy matching and tree pruning
  selection.rs   expansion and keyboard selection state
  render.rs      rows, icons, colors and viewport
  launcher.rs    optional two-step sequence detector
```

Core state:

```rust
struct State {
    sessions: Vec<SessionNode>,
    running_commands: HashMap<u32, Vec<String>>,
    pane_cwds: HashMap<u32, PathBuf>,
    focused_pane: Option<u32>,
    query: String,
    visible_rows: Vec<Row>,
    selected: usize,
    expanded: HashSet<NodeKey>,
}
```

The model must be rebuilt safely when `TabUpdate`, `PaneUpdate` and
`SessionUpdate` arrive in different orders. Node identity should not depend on
display position.

## Data flow

```text
Zellij events
  ├─ SessionUpdate -> session/tab/pane snapshot
  ├─ TabUpdate     -> current-session tab metadata
  ├─ PaneUpdate    -> current-session pane metadata
  ├─ CommandChanged -> running-command cache
  └─ CwdChanged     -> pane CWD cache
            │
            ▼
        tree model
            │
      fuzzy filter
            │
            ▼
     visible row model
            │
            ▼
      rendered HUD pane
```

Navigation flows through one adapter so current-session focus and
cross-session focus use the same target abstraction.

## Permissions

Minimum expected permissions:

- `ReadApplicationState`
- `ChangeApplicationState`

If the optional launcher dynamically rewrites keybindings:

- `Reconfigure`
- `InterceptInput` only if the safer double-chord launcher proves insufficient.

No command execution permission is needed for live status because Zellij
provides the running command API directly.

## Refresh and performance

- Update state only on relevant Zellij events.
- Query missing command state once when the finder opens.
- Query missing CWD state once when the finder opens.
- Spinner animation runs only while the finder is visible and only when at
  least one visible pane is running.
- Recompute the fuzzy result only when the query or underlying tree changes.
- Large sessions should remain responsive with several hundred panes.

## Testing strategy

Unit tests:

- tree construction across current and other sessions;
- hidden level combinations;
- stable selection after refresh;
- fuzzy filtering preserves ancestors;
- icons for shell, editor, Codex, Claude, floating, exited and unknown;
- progress state, command and CWD detail rendering;
- detail-panel placement at narrow and wide widths;
- navigation target selection across sessions;
- configuration parsing and invalid values.

Integration tests with real Zellij:

- open/close floating pane;
- search and Enter navigation;
- switch to another live session and focus a pane;
- select a resurrectable session;
- verify `CommandChanged` icon updates;
- verify `CwdChanged` detail updates;
- verify the optional launcher sequence and timeout.

Visual fixtures:

- narrow and wide terminals;
- Nerd Font and plain mode;
- many sessions;
- long names and Unicode titles.

## Milestones

1. Research and design review.
2. Minimal finder with current-session tab/pane tree.
3. Cross-session tree and one-step focus.
4. Live command icons and HUD styling.
5. Configuration surface.
6. Shortcut launcher spike.
7. Integration verification and packaging.

## Open decisions

- Exact two-step shortcut reliability target.
- Whether the initial release includes both double-chord and literal sequenced
  launchers.
- Whether full nested split geometry is worth a later implementation.
- How to present unknown command state for non-current sessions without visual
  noise.
- How to present unknown CWD for non-current sessions without duplicating the
  command placeholder.
