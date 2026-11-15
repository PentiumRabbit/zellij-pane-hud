# Flat Pane Template HUD Design

Date: 2026-11-15

Status: approved for implementation planning

## Goal

Replace the collapsible session/tab/pane tree and detail panel with a compact,
flat pane picker. Every visible result is one pane on one terminal row. Rows use
a configurable template, colored state/type icons, relative directories,
fuzzy-match highlighting, and a full-width selected-row background.

## Interaction

- Each pane occupies exactly one row.
- Session and tab rows are not shown and branches cannot be folded.
- Up/Down and `Ctrl+p`/`Ctrl+n` move through pane rows.
- Left/Right no longer fold or expand anything.
- Enter focuses the selected pane and closes the HUD.
- Esc closes the HUD without navigation.
- The FIND/search input remains above the flat list.
- The selected pane details panel and all detail-related layout behavior are
  removed.

The default order remains current session first, then tab position, then pane
ID. A non-empty query sorts results by match score, then by the same stable
order.

## Default Row

The default format is:

```text
{progress} {icon} {title}  {dir}  {session}/{tab}  {command}  #{pane_id}
```

For example:

```text
◓ ❉ fix search rendering  crates/tree-hud-core  work/code  codex --full-auto  #3
○ >_ zsh  .  work/shell  zsh  #7
✖ 󰈙 old editor  workspace  work/docs  nvim notes.md  #8
```

The progress field is deliberately near the beginning so running state is easy
to scan.

## Template Language

Configuration uses `row_format`. The value is parsed once when the plugin
loads.

Supported fields:

| Placeholder | Value |
|---|---|
| `{progress}` | Colored running spinner, idle, exited, or unknown status |
| `{icon}` | Colored pane/command type icon |
| `{title}` | Pane title |
| `{dir}` | Pane CWD relative to the focused pane's CWD |
| `{session}` | Session name |
| `{tab}` | Tab name |
| `{command}` | Full cached foreground command, or initial command fallback |
| `{pane_id}` | Numeric terminal pane ID |

Rules:

- Literal text, spacing, and separators are preserved.
- `{{` and `}}` emit literal braces.
- Unknown placeholders remain literal so user mistakes are visible.
- If parsing fails, the default format is used.
- Search fields are derived from the placeholders present in the active
  template. Hidden fields do not participate in matching.
- `{progress}` and `{icon}` are generated visual fields and do not contribute
  search text.

## Relative Directory

The base directory is the current focused pane's CWD.

- If a pane CWD equals the base, display `.`.
- If it is a descendant of the base, display the relative path.
- Otherwise display the absolute path.
- If CWD is unavailable, display `?`.

The calculation uses path components rather than string-prefix matching so
`/work/app` is not treated as the parent of `/work/application`.

## Search and Highlighting

Fuzzy mode returns both a score and the matched character positions.
Substring mode returns the contiguous matched character range.

- Only text fields present in `row_format` are searched. The default format
  therefore searches title, relative directory, session, tab, command, and
  pane ID.
- Text fields added to the format, such as `{command}` or `{session}`, become
  searchable automatically.
- Matching is case-insensitive across UTF-8 text.
- Match positions are tracked as character indices, not byte offsets.
- Fuzzy matching is also performed across the visible searchable fields in
  template order, allowing query characters to be separated by field
  boundaries.
- Matched characters use bold yellow foreground.
- A row is included if any visible search field matches.
- If the same row has matches in multiple fields, all visible matched segments
  are highlighted.

## Colors and Icons

Colors remain enabled in both Nerd and plain glyph modes. The glyph mode only
changes icon characters.

Semantic roles are mapped to the active Zellij theme through
`ModeUpdate(ModeInfo)`:

| State/Field | Color |
|---|---|
| focused | Zellij green emphasis |
| running command | Zellij cyan emphasis |
| idle shell | normal text color |
| Codex | Zellij magenta emphasis |
| Claude | Zellij orange emphasis |
| editor | theme blue |
| exited | Zellij error color |
| unknown | normal text color |
| title | normal foreground |
| directory | theme blue |
| fuzzy highlight | bold theme yellow |
| selected row foreground/background | Zellij `text_selected` |

The plugin keeps semantic roles but no longer hard-codes a palette after
receiving `ModeUpdate`. Missing optional theme slots fall back to the normal
text color.

Selected-row background spans the entire terminal width. State colors and match
foregrounds remain visible on top of that background.

`{progress}` renders:

- a four-frame cyan spinner while a foreground command is running;
- a gray idle mark for a shell at the prompt;
- a red exit mark for an exited pane;
- a dim unknown mark when status cannot be determined.

`{icon}` renders the pane type or command category independently from progress.

## Configuration

The active configuration becomes:

```kdl
row_format "{progress} {icon} {title}  {dir}  {session}/{tab}  {command}  #{pane_id}"
scope "all"                    // current | all
icons "nerd"                   // nerd | plain
search_mode "fuzzy"            // fuzzy | substring
spinner_fps "4"
```

Legacy tree/detail keys remain accepted but no longer affect layout. Invalid
`row_format` values fall back to the default. The existing `Ctrl+o`, then `g`
launch sequence is unchanged.

## Rendering Architecture

Introduce a parsed template model:

```rust
enum TemplatePart {
    Literal(String),
    Field(Field),
}

enum Field {
    Progress,
    Icon,
    Title,
    Dir,
    Session,
    Tab,
    Command,
    PaneId,
}
```

At load time:

1. Parse and validate `row_format`.
2. Derive the searchable field set.
3. Cache both in the finder configuration.

Rendering evaluates the cached parts into styled spans:

```rust
struct Span {
    text: String,
    foreground: ForegroundColor,
    background: Option<BackgroundColor>,
    bold: bool,
}
```

Plain-width truncation and padding operate on span text, never on ANSI escape
bytes. Selected-row background and match highlighting are applied after width
truncation.

## Performance

- A template is parsed once per plugin load, not once per row or render.
- Search text contains only fields present in the template.
- Fuzzy positions are calculated during the existing search rebuild; no second
  search pass is added.
- Rendering reads only the visible viewport slice.
- Selection and spinner animation reuse cached rows.
- Search rows are not rebuilt merely because the spinner frame changes.
- Progress/status updates rebuild rows only when their searchable fields are
  present in the template.

The expected working set remains tens to a few hundred panes. Existing
2,000-pane state-update and 10,000-row viewport tests must continue to pass.

## Error Handling

- Invalid or empty formats use the default.
- Unknown placeholders render literally.
- Unterminated braces render literally.
- Missing title falls back to `pane <id>`.
- Missing command, CWD, tab name, or session values use `?`.
- ANSI styling is generated by the core renderer; configuration cannot inject
  raw escape sequences.
- Narrow terminals truncate the assembled row with an ellipsis while preserving
  width safety.

## Testing

Required regression coverage:

1. Template parser handles all fields, literals, escaped braces, unknowns, and
   invalid input fallback.
2. Flat rows contain each pane exactly once, with no session/tab/fold rows.
3. Relative CWD handles equal, descendant, sibling-prefix, absolute fallback,
   and missing values.
4. Fuzzy and substring matches expose UTF-8-safe character positions.
5. Search corpus changes with the template fields.
6. Match highlighting marks only matched characters.
7. Each state/type icon uses the expected semantic color.
8. Selected-row background covers the full row width and layers correctly with
   state and match colors.
9. Long rows remain width-safe and never leak ANSI bytes into width accounting.
10. Viewport rendering remains bounded at 10,000 rows.
11. Template parsing does not repeat per render.
12. Existing key navigation, navigation targets, spinner behavior, and render
    feedback-loop regressions continue to pass.

Snapshot coverage must include:

- idle and running panes;
- fuzzy-highlighted title and directory;
- selected background;
- Nerd and plain glyph modes;
- a custom format using session/tab/command placeholders.

## Out of Scope

- Numeric percentage progress.
- Session/tab hierarchy rows.
- Collapsing or expanding branches.
- Configurable arbitrary ANSI styles.
- User-defined color palettes.
- Nested split geometry.
