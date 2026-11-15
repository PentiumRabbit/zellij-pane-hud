# Flat Pane Template HUD Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the collapsible tree/details HUD with a flat pane picker that supports configurable row templates, colored state/type icons, relative directories, fuzzy-match highlighting, and full-row selection.

**Architecture:** Add a parsed row-template model whose text fields also define the search corpus. Flatten the row model to one pane per row, return UTF-8 character match positions from the search layer, and render styled spans before converting them to ANSI with width-safe truncation and padding.

**Tech Stack:** Rust 2021, `unicode-width 0.2.2`, `zellij-tile 0.45.1`, `wasm32-wasip1`, Zellij plugin snapshot 0.2.3.

**Spec:** `docs/superpowers/specs/2026-11-15-flat-pane-template-design.md`

## Global Constraints

- Target Zellij 0.45.1 and `wasm32-wasip1`.
- Every pane is exactly one row; no session/tab hierarchy rows and no folding.
- No detail panel is rendered.
- `row_format` is parsed once per plugin load and cached.
- Search only fields present in `row_format`.
- Match positions are UTF-8 character indices.
- Width calculations use visible text, never ANSI escape bytes.
- Use Zellij's native `Ctrl+o`, then `g` launch sequence.
- Do not add command/CWD polling.
- Preserve the bounded retry, render-feedback-loop, and idle-spinner fixes.
- `cargo test --workspace`, formatting, clippy, WASM build, KDL check, and snapshot verification must pass.

---

### Task 1: Row Template Parser and Configuration

**Files:**
- Create: `crates/pane-hud-core/src/template.rs`
- Create: `crates/pane-hud-core/tests/template_test.rs`
- Modify: `crates/pane-hud-core/src/lib.rs`
- Modify: `crates/pane-hud-core/src/config.rs`
- Modify: `crates/pane-hud-core/tests/config_test.rs`

**Interfaces:**

Produces:

```rust
pub const DEFAULT_ROW_FORMAT: &str =
    "{progress} {icon} {title}  {dir}  {session}/{tab}  {command}  #{pane_id}";

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TemplateField {
    Progress,
    Icon,
    Title,
    Dir,
    Session,
    Tab,
    Command,
    PaneId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TemplatePart {
    Literal(String),
    Field(TemplateField),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RowTemplate {
    parts: Vec<TemplatePart>,
    search_fields: Vec<TemplateField>,
}

impl RowTemplate {
    pub fn parse(input: &str) -> Self;
    pub fn parts(&self) -> &[TemplatePart];
    pub fn search_fields(&self) -> &[TemplateField];
}

impl Default for RowTemplate {
    fn default() -> Self {
        Self::parse(DEFAULT_ROW_FORMAT)
    }
}
```

`Config` gains:

```rust
pub row_format: RowTemplate,
```

- [ ] **Step 1: Write failing template parser tests**

Create `crates/pane-hud-core/tests/template_test.rs`:

```rust
use pane_hud_core::template::{RowTemplate, TemplateField, TemplatePart};

#[test]
fn parses_fields_literals_and_escaped_braces() {
    let template = RowTemplate::parse(
        "{progress} {icon}|{title} {dir} {{x}} {session} {tab} {command} #{pane_id}",
    );

    assert_eq!(
        template.parts(),
        &[
            TemplatePart::Field(TemplateField::Progress),
            TemplatePart::Literal(" ".into()),
            TemplatePart::Field(TemplateField::Icon),
            TemplatePart::Literal("|".into()),
            TemplatePart::Field(TemplateField::Title),
            TemplatePart::Literal(" ".into()),
            TemplatePart::Field(TemplateField::Dir),
            TemplatePart::Literal(" {x} ".into()),
            TemplatePart::Field(TemplateField::Session),
            TemplatePart::Literal(" ".into()),
            TemplatePart::Field(TemplateField::Tab),
            TemplatePart::Literal(" ".into()),
            TemplatePart::Field(TemplateField::Command),
            TemplatePart::Literal(" #".into()),
            TemplatePart::Field(TemplateField::PaneId),
        ]
    );
}

#[test]
fn derives_search_fields_only_from_text_placeholders() {
    let template = RowTemplate::parse("{progress} {icon} {title}  {dir} {dir}");

    assert_eq!(
        template.search_fields(),
        &[TemplateField::Title, TemplateField::Dir]
    );
}

#[test]
fn invalid_unknown_and_unterminated_tokens_remain_literal() {
    assert_eq!(
        RowTemplate::parse("{unknown} {title").parts(),
        &[TemplatePart::Literal("{unknown} {title".into())]
    );
}

#[test]
fn empty_format_uses_the_default_template() {
    assert_eq!(RowTemplate::parse(""), RowTemplate::default());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p pane-hud-core --test template_test
```

Expected: compilation fails because `pane_hud_core::template` does not exist.

- [ ] **Step 3: Implement the minimal parser**

In `template.rs`, parse one character at a time. Merge adjacent literal text
into one `TemplatePart::Literal`. For `{{`, append `{`; for `}}`, append `}`.
If a `{...}` body is not a known field, preserve the entire token as literal.
If no closing `}` exists, preserve the remaining text as literal.

Derive `search_fields` in first-occurrence order from `Title`, `Dir`, `Session`,
`Tab`, `Command`, and `PaneId`. Deduplicate repeated fields. Exclude `Progress`
and `Icon`.

Expose the module from `lib.rs`:

```rust
pub mod template;
```

- [ ] **Step 4: Verify parser tests pass**

Run:

```bash
cargo test -p pane-hud-core --test template_test
```

Expected: PASS.

- [ ] **Step 5: Write a failing configuration test**

Add to `config_test.rs`:

```rust
#[test]
fn row_format_is_parsed_and_invalid_values_fall_back() {
    let custom = Config::from_map(&BTreeMap::from([(
        "row_format".into(),
        "{icon} {session}/{tab} {title} {command}".into(),
    )]));
    assert_eq!(
        custom.row_format.search_fields(),
        &[
            TemplateField::Session,
            TemplateField::Tab,
            TemplateField::Title,
            TemplateField::Command,
        ]
    );

    let empty = Config::from_map(&BTreeMap::from([("row_format".into(), String::new())]));
    assert_eq!(empty.row_format, RowTemplate::default());
}
```

Import `RowTemplate` and `TemplateField`.

- [ ] **Step 6: Run the configuration test to verify it fails**

Run:

```bash
cargo test -p pane-hud-core --test config_test row_format_is_parsed_and_invalid_values_fall_back
```

Expected: compilation fails because `Config::row_format` does not exist.

- [ ] **Step 7: Add `row_format` to `Config`**

In `Config::default`, use `RowTemplate::default()`.

In `Config::from_map`, parse a non-empty trimmed value with
`RowTemplate::parse`. Empty or absent values keep the default. Preserve legacy
keys without using them for layout.

- [ ] **Step 8: Run focused and workspace tests**

Run:

```bash
cargo test -p pane-hud-core --test template_test
cargo test -p pane-hud-core --test config_test
cargo test --workspace
```

Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add crates/pane-hud-core/src/template.rs \
  crates/pane-hud-core/src/lib.rs \
  crates/pane-hud-core/src/config.rs \
  crates/pane-hud-core/tests/template_test.rs \
  crates/pane-hud-core/tests/config_test.rs
git commit -m "feat: add configurable row templates"
```

---

### Task 2: UTF-8 Match Positions

**Files:**
- Modify: `crates/pane-hud-core/src/search.rs`
- Modify: `crates/pane-hud-core/tests/search_test.rs`

**Interfaces:**

Produces:

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchResult {
    pub score: MatchScore,
    pub positions: Vec<usize>,
}

pub fn match_subsequence(query: &str, text: &str) -> Option<MatchResult>;
pub fn match_substring(query: &str, text: &str) -> Option<MatchResult>;
```

Existing `score_subsequence` and `score_substring` remain wrappers returning
only `MatchScore`.

- [ ] **Step 1: Write failing match-position tests**

Add to `search_test.rs`:

```rust
#[test]
fn fuzzy_matches_return_utf8_character_positions() {
    let result = match_subsequence("树h", "项目树HUD").expect("match");

    assert_eq!(result.positions, vec![2, 3]);
}

#[test]
fn substring_matches_return_the_contiguous_character_range() {
    let result = match_substring("hud", "项目 HUD 面板").expect("match");

    assert_eq!(result.positions, vec![3, 4, 5]);
}
```

Import the new functions.

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p pane-hud-core --test search_test
```

Expected: compilation fails because match-position APIs do not exist.

- [ ] **Step 3: Implement match results**

Keep the current scoring algorithm. In the subsequence scanner, the positions
already tracked are character indices; return them in `MatchResult`.

For substring mode, compute the start in characters from the byte offset:

```rust
let char_start = text[..byte_start].chars().count();
let positions = (char_start..char_start + query.chars().count()).collect();
```

Normalize query/text lowercase before scanning, matching current behavior.

- [ ] **Step 4: Verify focused and full tests pass**

Run:

```bash
cargo test -p pane-hud-core --test search_test
cargo test --workspace
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/pane-hud-core/src/search.rs \
  crates/pane-hud-core/tests/search_test.rs
git commit -m "feat: return fuzzy match positions"
```

---

### Task 3: Flat Pane Rows, Relative Directories, and Searchable Template Fields

**Files:**
- Rewrite: `crates/pane-hud-core/src/rows.rs`
- Rewrite: `crates/pane-hud-core/tests/rows_test.rs`
- Modify: `crates/pane-hud-core/src/icons.rs`
- Modify: `crates/pane-hud-core/tests/icons_test.rs`
- Modify: `crates/pane-hud-core/src/app.rs`

**Interfaces:**

Produces:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgressState {
    Running,
    Idle,
    Exited,
    Unknown,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FieldMatches {
    pub session: Vec<usize>,
    pub tab: Vec<usize>,
    pub title: Vec<usize>,
    pub dir: Vec<usize>,
    pub command: Vec<usize>,
    pub pane_id: Vec<usize>,
}

impl FieldMatches {
    pub fn get(&self, field: TemplateField) -> &[usize];
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Row {
    pub key: NodeKey,
    pub session: String,
    pub tab_name: String,
    pub tab_position: usize,
    pub title: String,
    pub dir: String,
    pub command: String,
    pub pane: LivePane,
    pub is_current_session: bool,
    pub progress: ProgressState,
    pub score: u32,
    pub matches: FieldMatches,
}

pub struct Rows {
    pub rows: Vec<Row>,
}

impl Rows {
    pub fn build(
        world: &World,
        config: &Config,
        query: &str,
        values: &PaneValueCache,
    ) -> Self;
}
```

`Rows::build` emits only pane rows. It does not accept expansion state.

- [ ] **Step 1: Write failing flat-row and relative-dir tests**

Replace `rows_test.rs` with tests covering:

```rust
#[test]
fn emits_each_pane_once_without_session_or_tab_rows() {
    // Two sessions, two tabs, three panes.
    let rows = Rows::build(&world, &config, "", &values);

    assert_eq!(rows.rows.len(), 3);
    assert!(rows.rows.iter().all(|row| row.pane.id > 0));
    assert_eq!(
        rows.rows.iter().map(|row| row.pane.id).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
}

#[test]
fn relative_dir_handles_equal_descendant_and_sibling_prefix_paths() {
    // Focused pane cwd: /work/app
    assert_eq!(dir_for("/work/app"), ".");
    assert_eq!(dir_for("/work/app/src"), "src");
    assert_eq!(dir_for("/work/application"), "/work/application");
    assert_eq!(dir_for_missing(), "?");
}

#[test]
fn template_fields_control_the_search_corpus() {
    // Title differs from command; hidden command must not match.
    let title_only = Config::from_map(&BTreeMap::from([(
        "row_format".into(),
        "{title} {dir}".into(),
    )]));
    let with_command = Config::from_map(&BTreeMap::from([(
        "row_format".into(),
        "{title} {dir} {command}".into(),
    )]));

    assert!(Rows::build(&world, &title_only, "cargo", &values).rows.is_empty());
    assert_eq!(Rows::build(&world, &with_command, "cargo", &values).rows.len(), 1);
}
```

Use literal fixtures for paths and commands. Do not compute expectations with
the helper under test.

- [ ] **Step 2: Run row tests to verify they fail**

Run:

```bash
cargo test -p pane-hud-core --test rows_test
```

Expected: old tree-row behavior violates flat counts and/or APIs do not compile.

- [ ] **Step 3: Rewrite rows as a flat pane list**

For each live session allowed by `scope`, each tab, and each non-plugin
selectable pane:

1. Build the display title, falling back to `pane <id>`.
2. Resolve the focused current-session pane CWD as the base path.
3. Compute `{dir}` from path components:

```rust
let dir = match (pane_cwd, base_cwd) {
    (None, _) => "?".to_owned(),
    (Some(path), None) => path.display().to_string(),
    (Some(path), Some(base)) if path == base => ".".to_owned(),
    (Some(path), Some(base)) => path
        .strip_prefix(base)
        .map(|relative| relative.display().to_string())
        .unwrap_or_else(|_| path.display().to_string()),
};
```

4. Build independent `session`, `tab`, `title`, `dir`, `command`, and pane-ID
   strings.
5. Score only fields listed by `config.row_format.search_fields()`.
6. Save positions from the best scoring result per field.
7. Compute `ProgressState` from `is_foreground`, executable category, and
   exited state.
8. Sort by score descending, then original stable traversal order.

- [ ] **Step 4: Add progress/type separation in icons**

Keep `PaneVisualState` for the type/category icon. Add:

```rust
pub fn progress_state(
    pane: &LivePane,
    command: Option<&[String]>,
    is_foreground: Option<bool>,
) -> ProgressState;
```

`is_foreground=true` means Running even for a nested shell.
`is_foreground=false` means Idle. Exited takes precedence. Missing command
state is Unknown.

- [ ] **Step 5: Update Finder internals**

Remove expansion state and tree-only methods. `Rows::build` no longer receives
expansion. Keep stable key restoration and dirty-count behavior.

Left/Right actions become no-ops in the application layer; do not rebuild rows.

- [ ] **Step 6: Run focused and workspace tests**

Run:

```bash
cargo test -p pane-hud-core --test rows_test
cargo test -p pane-hud-core --test icons_test
cargo test -p pane-hud-core --test app_test
cargo test --workspace
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/pane-hud-core/src/rows.rs \
  crates/pane-hud-core/src/icons.rs \
  crates/pane-hud-core/src/app.rs \
  crates/pane-hud-core/tests/rows_test.rs \
  crates/pane-hud-core/tests/icons_test.rs \
  crates/pane-hud-core/tests/app_test.rs
git commit -m "feat: build flat pane rows"
```

---

### Task 4: Width-Safe Styled Span Primitives

**Files:**
- Create: `crates/pane-hud-core/src/style.rs`
- Create: `crates/pane-hud-core/tests/style_test.rs`
- Modify: `crates/pane-hud-core/src/lib.rs`

**Interfaces:**

Produces in `style.rs`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ForegroundColor {
    Default,
    Gray,
    Green,
    Cyan,
    Magenta,
    Orange,
    Blue,
    Red,
    Yellow,
    DarkGray,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StyledSpan {
    pub text: String,
    pub foreground: ForegroundColor,
    pub selected_background: bool,
    pub bold: bool,
}

pub fn truncate_spans(spans: Vec<StyledSpan>, width: usize) -> Vec<StyledSpan>;
pub fn pad_spans(spans: Vec<StyledSpan>, width: usize) -> Vec<StyledSpan>;
pub fn spans_to_ansi(spans: &[StyledSpan], color: bool) -> String;
```

- [ ] **Step 1: Write failing Unicode width and style tests**

Create `style_test.rs`:

```rust
use pane_hud_core::style::{
    pad_spans, spans_to_ansi, truncate_spans, ForegroundColor, StyledSpan,
};

fn span(text: &str, foreground: ForegroundColor) -> StyledSpan {
    StyledSpan {
        text: text.into(),
        foreground,
        selected_background: false,
        bold: false,
    }
}

#[test]
fn truncation_is_unicode_width_safe_and_adds_an_ellipsis() {
    let spans = vec![span("项目abcdef", ForegroundColor::Default)];
    let truncated = truncate_spans(spans, 6);
    let rendered = spans_to_ansi(&truncated, false);

    assert_eq!(rendered, "项目a…");
}

#[test]
fn padding_uses_visible_width_not_byte_length() {
    let spans = pad_spans(
        vec![span("项目", ForegroundColor::Default)],
        6,
    );

    assert_eq!(spans_to_ansi(&spans, false), "项目  ");
}

#[test]
fn ansi_combines_foreground_bold_and_selected_background() {
    let spans = vec![StyledSpan {
        text: "hit".into(),
        foreground: ForegroundColor::Yellow,
        selected_background: true,
        bold: true,
    }];

    assert_eq!(
        spans_to_ansi(&spans, true),
        "\u{1b}[1;38;5;226;48;5;24mhit\u{1b}[0m"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p pane-hud-core --test style_test
```

Expected: compilation fails because `pane_hud_core::style` does not exist.

- [ ] **Step 3: Implement span width and ANSI conversion**

Use `UnicodeWidthStr` for complete spans and `UnicodeWidthChar` when splitting a
span during truncation.

Color mappings:

```rust
Default => 252
Gray => 245
Green => 46
Cyan => 51
Magenta => 201
Orange => 208
Blue => 75
Red => 196
Yellow => 226
DarkGray => 240
```

Selected background maps to `48;5;24`. When `color=false`, emit text only.
When an item uses default foreground and no bold/background, emit the item
text without an SGR wrapper.

Expose `pub mod style;` from `lib.rs`.

- [ ] **Step 4: Verify style tests pass**

Run:

```bash
cargo test -p pane-hud-core --test style_test
cargo test -p pane-hud-core
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/pane-hud-core/src/style.rs \
  crates/pane-hud-core/src/lib.rs \
  crates/pane-hud-core/tests/style_test.rs
git commit -m "feat: add width-safe styled spans"
```

---

### Task 5: Flat Renderer, Finder, and Zellij Integration

**Files:**
- Rewrite: `crates/pane-hud-core/src/render.rs`
- Rewrite: `crates/pane-hud-core/tests/render_test.rs`
- Modify: `crates/pane-hud-core/src/app.rs`
- Modify: `crates/pane-hud-core/src/icons.rs`
- Modify: `crates/pane-hud-core/tests/app_test.rs`
- Modify: `crates/pane-hud-core/src/lib.rs`
- Modify: `crates/pane-hud-plugin/src/plugin.rs`
- Modify: `crates/pane-hud-plugin/src/adapter.rs`
- Modify: `crates/pane-hud-plugin/tests/adapter_test.rs`
- Delete: `crates/pane-hud-core/src/details.rs`
- Delete: `crates/pane-hud-core/tests/details_test.rs`

**Interfaces:**

Produces:

```rust
pub fn render_frame(
    rows: &Rows,
    selected: Option<usize>,
    cols: usize,
    height: usize,
    query: &str,
    config: &Config,
    spinner_tick: u64,
    options: RenderOptions,
) -> RenderFrame;
```

Consumes `RowTemplate`, `Row`, `ProgressState`, `FieldMatches`, and
`style::{StyledSpan, ForegroundColor, truncate_spans, pad_spans,
spans_to_ansi}`.

- [ ] **Step 1: Write failing renderer tests with concrete rows**

Create a helper in `render_test.rs`:

```rust
fn row(id: u32, title: &str, dir: &str, command: &str) -> Row {
    let key = NodeKey::Pane {
        session: "work".into(),
        tab_id: 11,
        pane_id: id,
    };
    Row {
        key: key.clone(),
        session: "work".into(),
        tab_name: "code".into(),
        tab_position: 0,
        title: title.into(),
        dir: dir.into(),
        command: command.into(),
        pane: LivePane {
            key,
            id,
            title: title.into(),
            is_focused: id == 1,
            is_floating: false,
            is_fullscreen: false,
            is_suppressed: false,
            exited: false,
            initial_command: Some(command.into()),
        },
        is_current_session: true,
        progress: ProgressState::Running,
        score: 100,
        matches: FieldMatches {
            title: vec![0],
            ..FieldMatches::default()
        },
    }
}

fn render_one(selected: bool, query: &str) -> String {
    let config = Config::default();
    let panes = Rows {
        rows: vec![row(1, "src renderer", "src/ui", "codex --full-auto")],
    };
    let frame = render_frame(
        &panes,
        selected.then_some(0),
        60,
        12,
        query,
        &config,
        1,
        RenderOptions::default(),
    );
    frame.lines.join("\n")
}
```

Tests:

```rust
#[test]
fn progress_and_type_icons_use_semantic_colors() {
    let rendered = render_one(false, "");
    assert!(rendered.contains("\u{1b}[38;5;51m"));
    assert!(rendered.contains("\u{1b}[38;5;201m"));
}

#[test]
fn fuzzy_match_segments_are_bold_yellow() {
    let rendered = render_one(false, "s");
    assert!(rendered.contains("\u{1b}[1;38;5;226m"));
}

#[test]
fn selected_rows_have_a_full_width_dark_background() {
    let rendered = render_one(true, "");
    assert!(rendered.contains("48;5;24"));
    assert_eq!(frame_visible_width_without_ansi(&rendered, 60), true);
}

#[test]
fn detail_panel_text_is_not_rendered() {
    let rendered = render_one(false, "");
    assert!(!rendered.contains("selected\n"));
    assert!(!rendered.contains("command  "));
}
```

Define `frame_visible_width_without_ansi` as a test helper that strips SGR
sequences and asserts each line's `UnicodeWidthStr <= 60`.

- [ ] **Step 2: Run render tests to verify they fail**

Run:

```bash
cargo test -p pane-hud-core --test render_test
```

Expected: the old renderer still requires details/layout arguments and emits
tree/detail text.

- [ ] **Step 3: Implement template evaluation**

For every visible row:

- Walk `config.row_format.parts()`.
- Emit literals with `ForegroundColor::Default`.
- Emit title/session/tab/command/pane ID with default foreground.
- Emit dir with blue.
- Emit progress with state-specific glyph and color.
- Emit icon with category glyph and color.
- For text fields, use `FieldMatches::get` and split at match positions so
  matched spans are bold yellow.
- Merge adjacent spans with identical styles.

- [ ] **Step 4: Implement the flat frame**

The frame is exactly:

```text
top border
FIND search line
divider
visible rows (height - 5)
footer
bottom border
```

Each row is truncated and padded to `cols`, then converted to ANSI. The
selected row marks every span, including padding, with
`selected_background=true`. There is no details section.

- [ ] **Step 5: Update Finder and plugin call sites**

- Remove `details()`, `expanded`, collapse, and expand behavior.
- Keep stable key-based selection.
- Left/Right return `InputIntent::Ignored`.
- Update `render_frame` call to omit details/values.
- Keep cursor deduplication and bounded detail retries.
- Spinner scheduling checks `ProgressState::Running`.
- `find` does not rebuild rows.

- [ ] **Step 6: Add integration-decision tests**

Add to plugin tests:

```rust
#[test]
fn left_and_right_keys_do_not_request_render() {
    let mut plugin = PaneHudPlugin::default();
    assert!(!plugin.on_key(KeyWithModifier::new(BareKey::Left)));
    assert!(!plugin.on_key(KeyWithModifier::new(BareKey::Right)));
}
```

Add to `app_test.rs`:

```rust
#[test]
fn spinner_and_vertical_selection_reuse_rows() {
    let mut finder = Finder::new(Config::default());
    finder.set_snapshot(world_with(vec![pane(1, true), pane(2, false)]));
    finder.refresh();
    let rebuilds = finder.rebuild_count();

    finder.advance_spinner();
    finder.selection_down();
    finder.selection_up();
    finder.refresh();

    assert_eq!(finder.rebuild_count(), rebuilds);
}
```

- [ ] **Step 7: Run renderer, core, and plugin tests**

Run:

```bash
cargo test -p pane-hud-core --test render_test
cargo test -p pane-hud-core
cargo test -p pane-hud-plugin
```

Expected: PASS.

- [ ] **Step 8: Remove details module and tests**

```bash
git rm crates/pane-hud-core/src/details.rs \
  crates/pane-hud-core/tests/details_test.rs
```

Remove `pub mod details;` from `lib.rs`.

- [ ] **Step 9: Run workspace tests**

Run:

```bash
cargo test --workspace
```

Expected: PASS.

- [ ] **Step 10: Commit**

```bash
git add crates/pane-hud-core/src/render.rs \
  crates/pane-hud-core/src/app.rs \
  crates/pane-hud-core/src/icons.rs \
  crates/pane-hud-core/src/lib.rs \
  crates/pane-hud-core/tests/render_test.rs \
  crates/pane-hud-core/tests/app_test.rs \
  crates/pane-hud-plugin/src/plugin.rs \
  crates/pane-hud-plugin/src/adapter.rs \
  crates/pane-hud-plugin/tests/adapter_test.rs
git commit -m "feat: integrate flat colored pane rows"
```

---

### Task 6: Documentation, Example Config, Snapshots, and Final Verification

**Files:**
- Modify: `README.md`
- Modify: `examples/config.kdl`
- Modify: `docs/DESIGN.md`
- Modify: `docs/RESEARCH.md`
- Modify: `docs/superpowers/specs/2026-11-14-zellij-pane-hud-core-design.md`
- Modify: `tests/snapshot/smoke.yaml`
- Create: `tests/snapshot/template.yaml`

**Interfaces:**

Consumes all previous behavior. Produces user-facing documentation and
multi-state visual fixtures.

- [ ] **Step 1: Update the example config**

```kdl
plugins {
    pane-hud location="file:/absolute/path/dist/pane-hud.wasm" {
        scope "all"
        row_format "{progress} {icon} {title}  {dir}  {session}/{tab}  {command}  #{pane_id}"
        icons "nerd"
        search_mode "fuzzy"
        spinner_fps "4"
    }
}
```

Document additional examples:

```text
{progress} {icon} {title}  {dir}  {session}/{tab}  {command}  #{pane_id}
{icon} {session}/{tab} {title}  {dir}  {command}
{progress} {title} [{dir}]
```

- [ ] **Step 2: Update README behavior**

Document:

- flat pane-only list;
- no folding and no detail panel;
- formats and fields;
- template-scoped search;
- colors and match/selection highlighting;
- relative CWD rules;
- the `Ctrl+o`, then `g` launcher.

- [ ] **Step 3: Add snapshot fixtures**

`smoke.yaml` covers idle, running, exited, selected, long paths, and Unicode.
`template.yaml` sets:

```yaml
config:
  icons: plain
  row_format: "{progress} {icon} {session}/{tab} {title}  {dir}  {command}"
```

Snapshot checks must assert:

- `FIND` is present;
- no `selected` detail heading exists;
- at least one semantic icon color is present;
- at least one match-highlight SGR sequence is present;
- at least one selected-background SGR sequence is present.

- [ ] **Step 4: Run all verification**

Run:

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
./scripts/build-wasm.sh
./scripts/check-snapshot.sh
zellij --config examples/config.kdl setup --check
```

Expected: every command exits 0.

- [ ] **Step 5: Run a real Zellij smoke test**

In a fresh session:

1. `Esc` dismisses About.
2. `Ctrl+o`, then `g` opens the HUD.
3. Type a fuzzy query present in title or dir.
4. Confirm only matching text is highlighted.
5. Confirm the selected row has a full-width background.
6. Confirm Left/Right do nothing.
7. Confirm Enter focuses the selected pane and closes.
8. Confirm idle output stops.

- [ ] **Step 6: Commit**

```bash
git add README.md examples/config.kdl docs/DESIGN.md docs/RESEARCH.md \
  docs/superpowers/specs/2026-11-14-zellij-pane-hud-core-design.md \
  tests/snapshot/smoke.yaml tests/snapshot/template.yaml scripts/check-snapshot.sh
git commit -m "docs: document flat template pane HUD"
```

---

## Self-Review

Spec coverage:

- Flat pane-only rows: Task 3.
- No details/folding: Tasks 3-5.
- Configurable template and field-derived search: Tasks 1 and 3.
- Relative CWD: Task 3.
- UTF-8 fuzzy positions and highlighting: Tasks 2 and 5.
- Colored progress/type icons: Tasks 3 and 5.
- Full selected background: Task 5.
- Performance and viewport bounds: Tasks 3-5.
- Documentation and visual verification: Task 6.

Type consistency:

- `RowTemplate::search_fields()` returns `&[TemplateField]`.
- `Rows::build` always takes `(world, config, query, values)`.
- `render_frame` always takes `(rows, selected, cols, height, query, config,
  spinner_tick, options)`.
- `ProgressState::Running` drives spinner scheduling and progress animation.
- `FieldMatches` stores character positions for every searchable text field.
