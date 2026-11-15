# Design

Status: implemented flat pane-template HUD.

## Product definition

`zellij-tree-hud` is a temporary floating pane picker for Zellij. It combines:

- the compact visual language of a HUD;
- search-first pane selection;
- one line per terminal pane;
- configurable row templates;
- colored progress and command-category icons;
- direct current-session and cross-session pane focus.

It is not a permanent sidebar and does not render a session/tab tree.

## Interaction

1. The user opens the configured floating plugin.
2. Every live selectable terminal pane is represented by exactly one row.
3. Typing filters rows using only text fields present in `row_format`.
4. Up/Down and `Ctrl+p`/`Ctrl+n` move through the flat result list.
5. Left/Right are ignored.
6. Enter focuses the selected pane, closes the HUD, and switches sessions when
   necessary.
7. Esc closes without navigation.

The focused pane is selected initially when `follow_current_session=true`.
Selection is restored by stable session/tab/pane identity across row rebuilds.

## Default row

```text
{progress} {icon} {title}  {dir}
```

Example:

```text
◓ ❉ fix search rendering  crates/tree-hud-core
○ >_ zsh                  .
✖ 󰈙 old editor            workspace
```

Progress comes first so running state is the easiest attribute to scan.

## Template language

The template is parsed once at plugin load into literal and field parts.

| Placeholder | Value |
|---|---|
| `{progress}` | Running spinner, idle, exited, or unknown progress |
| `{icon}` | Pane or command category icon |
| `{title}` | Pane title, or `pane <id>` when empty |
| `{dir}` | CWD relative to the focused pane CWD |
| `{session}` | Live session name |
| `{tab}` | Tab name |
| `{command}` | Full cached foreground command or initial fallback |
| `{pane_id}` | Numeric terminal pane ID |

Rules:

- literals and spacing are preserved;
- `{{` and `}}` emit literal braces;
- unknown placeholders remain literal;
- unterminated placeholders remain literal;
- empty or invalid formats fall back to the default template;
- search fields are derived from text placeholders in first-occurrence order;
- `{progress}` and `{icon}` are visual fields and are not searchable.

Search corpus examples:

```text
{progress} {icon} {title}  {dir}       -> title, dir
{icon} {session}/{tab} {title}         -> session, tab, title
{progress} {title} {command}           -> title, command
```

## Relative directories

The base directory is the focused current-session pane CWD.

```text
path == base             .
path is under base       relative path
path outside base        absolute path
missing path             ?
```

Comparison uses path components through `Path::strip_prefix`, so
`/work/app` is not treated as the parent of `/work/application`.

## Search and highlighting

Fuzzy mode uses a bounded subsequence scan. Substring mode uses direct
containment. Both return a score and UTF-8 character positions:

```rust
struct MatchResult {
    score: MatchScore,
    positions: Vec<usize>,
}
```

All visible fields are scored independently. A row is included when any
searchable field matches. Its score is the best field score, and positions for
every matching field are retained so the renderer can highlight each match.

Rows sort by score descending, then by stable traversal order:

```text
current session first
tab position
pane ID
```

Matched characters are bold yellow. Only matched characters change foreground
style.

## Icons, colors, and selection

`{progress}` and `{icon}` are independent fields.

Progress:

| State | Glyph | Color |
|---|---|---|
| Running | four-frame spinner | cyan |
| Idle shell | `○` | gray |
| Exited | `✖` | red |
| Unknown | `?` | dark gray |

Command category:

| Category | Color |
|---|---|
| focused | green |
| shell | gray |
| command | cyan |
| Codex | magenta |
| Claude | orange |
| editor | blue |
| floating/fullscreen/stacked | blue |
| unknown | dark gray |

The selected row uses a dark-blue (`48;5;24`) background across its complete
terminal width. Icon foregrounds and bold-yellow match highlights remain
visible on that background.

Directory text is blue. Other text fields use the normal foreground.

## Rendering

The frame is:

```text
top border
FIND/search line
divider
visible pane rows
footer
bottom border
```

Rendering evaluates cached template parts into styled spans:

```rust
struct StyledSpan {
    text: String,
    foreground: ForegroundColor,
    selected_background: bool,
    bold: bool,
}
```

Width rules:

- visible calculation uses `unicode-width`;
- ANSI escape bytes never participate in width calculation;
- assembled rows are truncated with an ellipsis when necessary;
- every row is padded to the exact inner width;
- the search line keeps a width-safe query tail and cursor position.

The selected viewport starts at `selected + 1 - visible_rows` rather than
walking all prior rows. A 10,000-row regression verifies bounded rendering.

## Architecture

```text
crates/tree-hud-core/
  app.rs       Finder state, selection, query, rebuild thresholds
  rows.rs      flat pane rows, relative dirs, field-scoped search
  input.rs     UTF-8 query editing and stable selection
  icons.rs     category classification and progress state
  model.rs     Zellij-independent world and value caches
  search.rs    scoring and UTF-8 match positions
  style.rs     width-safe styled spans and ANSI conversion
  template.rs  row template parser and field metadata
  render.rs    flat viewport renderer

crates/tree-hud-plugin/
  adapter.rs   Zellij event/key/action translation
  plugin.rs    plugin lifecycle, event subscriptions, navigation
```

The core has no Zellij host calls and remains host-testable.

## Data flow

```text
Zellij SessionUpdate / TabUpdate / PaneUpdate
  -> core World

CommandChanged / CwdChanged / bounded initial queries
  -> PaneValueCache

World + Config + Query + PaneValueCache
  -> flat Rows with scores and match positions

Rows + visible viewport
  -> styled spans
  -> ANSI frame
```

## Performance

- Parse `row_format` once, not once per row or render.
- Rebuild rows only when a dependency changes.
- Command updates invalidate rows only for `{command}`, `{progress}`, or
  `{icon}`; CWD updates invalidate only for `{dir}`.
- Search each visible text field once and retain all match positions.
- Render only the visible row slice.
- Reuse rows for selection and spinner animation.
- Do not poll command/CWD values.
- Bound failed command/CWD queries.
- Schedule spinner timers only while a visible current-session pane is running.

The target working set is tens to a few hundred panes. Existing regressions
cover 2,000-pane updates and a 10,000-row viewport.

## Navigation

- Current-session pane: `focus_terminal_pane`.
- Other live-session pane: `switch_session_with_focus` with the terminal pane
  target.
- Plugin and non-selectable panes are excluded.
- Suppressed panes remain selectable and can be restored during focus.

Command and CWD values for other sessions may be unavailable in Zellij 0.45.1;
their rows still support structural search and pane navigation.

## Configuration

```kdl
scope "all"                    // current | all
row_format "{progress} {icon} {title}  {dir}"
icons "nerd"                   // nerd | plain
search_mode "fuzzy"            // fuzzy | substring
spinner_fps "4"
follow_current_session "true"
```

Legacy tree/detail keys are accepted but ignored. Unknown keys are ignored.

## Shortcut strategy

Zellij 0.45.1 does not expose plugin-defined global keybindings. The supported
release path is ordinary Zellij KDL using the built-in session leader:

```text
Ctrl+o, then g
```

The final `g` binding is user-configurable. `Ctrl+Space` and `Ctrl+g` conflicted
with the local environment and are not registered.

## Error handling

- Missing title: `pane <id>`.
- Missing command, directory, tab name, or unavailable status: `?` where that
  field is textual.
- Invalid/empty template: default template.
- Unknown template tokens: literal text.
- Query/navigation errors never panic the plugin.
- Narrow terminals truncate safely rather than emitting malformed widths.

## Testing

Automated coverage includes:

- template parsing, escaping, fallback, and search-field derivation;
- flat pane-only row generation;
- relative path edge cases;
- UTF-8 fuzzy/substring positions;
- template-scoped matching;
- progress/category color semantics;
- match and selected-row styling layers;
- Unicode width safety;
- viewport bounds at 10,000 rows;
- stable selection and cached-row reuse;
- duplicate event suppression and bounded query retries;
- idle-spinner behavior;
- left/right no-op behavior;
- KDL validation, WASM build, snapshot output, and a real Zellij smoke test.
