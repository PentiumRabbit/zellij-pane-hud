# zellij-pane-switcher

[![CI](https://github.com/pr-g/zellij-pane-switcher/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/pr-g/zellij-pane-switcher/actions/workflows/ci.yml)

A floating, search-first pane picker for Zellij. Every terminal pane appears on
one compact row with progress, command type, title, relative directory,
session/tab, full command, and pane ID.

Type to fuzzy-filter the pane list, then press Enter to focus the selected pane
in the current or another live session.

```text
╭─ FIND ───────────────────────────────────────────────╮
│ › sr                                          Esc   │
├──────────────────────────────────────────────────────┤
│ ◓ ❉ src renderer  src/ui  work/code  codex --full-auto  #1 │
│ ○ 󰈙 editor notes  workspace/notes  work/code  nvim  #2      │
│ ✖ >_ old shell  /tmp/elsewhere  work/logs  zsh  #3           │
│ ↑↓ move  Enter jump  Esc close                      │
╰──────────────────────────────────────────────────────╯
```

## Features

- One terminal pane per row; no session/tab hierarchy, folding, or detail
  panel.
- Configurable row templates with title, relative directory, progress, icon,
  session, tab, command, and pane ID fields.
- Template-scoped fuzzy or substring search. Hidden fields do not participate
  in matching. Fuzzy queries may span multiple visible fields.
- UTF-8-safe match highlighting: matched characters are bold yellow.
- Semantic colors for running, idle, exited, unknown, Codex, Claude, editors,
  and other commands.
- Field-specific colors for titles, directories, sessions, tabs, commands, and
  pane IDs while keeping fuzzy matches bold yellow.
- Colors follow the active Zellij theme through `ModeUpdate` and
  `ModeInfo.style.colors`.
- Full-width theme-selected row background while preserving icon and match
  colors.
- Stable pane selection across Zellij updates and one-step cross-session pane
  focus.
- Nerd Font and plain-text icon modes.
- Viewport-bounded rendering and cached search rows. Selection movement,
  rendering, and spinner frames do not rebuild search results.

## Default row

```text
{progress} {icon} {title}  {dir}  {session}/{tab}  {command}  #{pane_id}
```

Supported fields:

| Field | Meaning |
|---|---|
| `{progress}` | Colored running, idle, exited, or unknown indicator |
| `{icon}` | Colored pane or command category icon |
| `{title}` | Pane title, falling back to `pane <id>` |
| `{dir}` | Pane CWD relative to the focused pane CWD |
| `{session}` | Live session name |
| `{tab}` | Tab name |
| `{command}` | Cached foreground command or initial-command fallback |
| `{pane_id}` | Numeric terminal pane ID |

Only text fields present in the active format are searchable. The complete
default format therefore searches title, relative directory, session, tab,
command, and pane ID. Removing a field also removes it from the search corpus.

`{{` and `}}` emit literal braces. Unknown or unterminated placeholders remain
literal. An empty format falls back to the default.

More examples:

```text
{progress} {icon} {title}  {dir}  {session}/{tab}  {command}  #{pane_id}
{icon} {session}/{tab} {title}  {dir}  {command}
{progress} {title} [{dir}]
```

`{dir}` uses path components, not string prefixes:

- equal to the focused pane CWD: `.`;
- a descendant: the relative path;
- an unrelated or sibling-prefix path: the absolute path;
- unavailable: `?`.

For example, `/work/app` is not treated as the parent of
`/work/application`.

## Performance model

The expected working set is tens to a few hundred panes.

- The template is parsed once when the plugin loads.
- Search runs only when the query, pane structure, relevant description data,
  or configured search fields change.
- Command/CWD changes rebuild rows only when the template renders or searches
  a dependent field.
- Rendering visits only the visible viewport rather than all preceding rows.
- Selection changes and spinner frames reuse cached rows.
- Command/CWD queries are event-driven with bounded retries. Idle shells do not
  schedule spinner timers.
- Regression coverage includes 2,000-pane updates and a 10,000-row viewport.

## Build

Requirements:

- Rust 1.93.0
- Zellij 0.45.1
- `wasm32-wasip1`

```bash
rustup target add wasm32-wasip1
./scripts/build-wasm.sh
```

The build script writes `dist/pane-switcher.wasm`.

Run the full verification suite:

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
./scripts/build-wasm.sh
./scripts/check-snapshot.sh
zellij --config examples/config.kdl setup --check
```

`check-snapshot.sh` requires `zellij-plugin-snapshot 0.2.3`.

GitHub Actions runs the same checks for pull requests and pushes to `main`,
then uploads `pane-switcher.wasm` as a workflow artifact. Pushing a `v*` tag creates
or updates a GitHub Release and publishes the WASM file plus its SHA-256
checksum.

## Zellij configuration

Zellij plugins do not define their own global shortcut API. Bind the plugin in
ordinary Zellij KDL. The example provides a direct `Ctrl+Shift+e` shortcut and
keeps `Ctrl+o`, then `g` as a fallback.

```kdl
plugins {
    pane-switcher location="https://github.com/pr-g/zellij-pane-switcher/releases/latest/download/pane-switcher.wasm" {
        scope "all"
        row_format "{progress} {icon} {title}  {dir}  {session}/{tab}  {command}  #{pane_id}"
        icons "nerd"
        search_mode "fuzzy"
        spinner_fps "4"
    }
}

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

The release URL automatically follows the latest published release. For local
development, replace it with
`file:/absolute/path/to/zellij-pane-switcher/dist/pane-switcher.wasm`.

Both shortcuts are configurable. `Ctrl+Shift+e` launches directly from normal
mode, while `Ctrl+o`, then `g` remains available through Zellij's session
leader. Restart Zellij or reload the config after changing either binding.

See [`examples/config.kdl`](examples/config.kdl) for a complete example.
Earlier `Ctrl+Space` and `Ctrl+g` candidates conflicted with the local setup and
are not registered by this plugin.

## Keyboard controls

| Key | Action |
|---|---|
| `Ctrl+Shift+e` | Open or focus the pane switcher directly |
| `Ctrl+o`, then `g` | Open or focus the pane switcher |
| Printable text | Edit the search query |
| Backspace | Delete one character |
| `Ctrl+w` | Delete the previous word |
| `Ctrl+u` | Clear the query |
| Up / `Ctrl+p` | Select previous pane row |
| Down / `Ctrl+n` | Select next pane row |
| PageUp / PageDown | Move by ten rows |
| Enter | Focus the selected pane and close |
| Esc | Close without navigation |
| Left / Right | No action |

## Configuration

Unknown keys are ignored. Invalid values fall back per field.

```kdl
scope "all"                    // current | all
row_format "{progress} {icon} {title}  {dir}  {session}/{tab}  {command}  #{pane_id}"
icons "nerd"                   // nerd | plain
search_mode "fuzzy"            // fuzzy | substring
spinner_fps "4"
follow_current_session "true"
```

Legacy tree/detail keys are still accepted so existing configs do not fail,
but they no longer affect layout or search. Use `row_format` to choose visible
and searchable fields.

`pane-switcher` can coexist with `zellij-hud`. Keep both plugin aliases and load
only the plugins that need background activation; the pane switcher itself is
launched on demand.

## Permissions and limitations

The plugin requests Zellij application-state access and permission to change
pane/session focus.

- Live command and CWD values are reliably available for panes in the current
  session. Other live sessions may display `?` or an unknown progress state.
- Only live selectable terminal panes are listed. Plugin panes, unselectable UI
  panes, and resurrectable sessions without live panes are excluded.
- Zellij 0.45.1 does not expose plugin-defined global shortcuts, so launching
  remains a user-configured Zellij keybinding.
- Zellij 0.45.1 `LaunchOrFocusPlugin` does not provide width or height options;
  the native floating-pane size is used.
- Zellij owns the floating pane frame. The plugin renders only content and does
  not draw a second inner border.

See [the design](docs/DESIGN.md) and
[technical research](docs/RESEARCH.md) for implementation details.
