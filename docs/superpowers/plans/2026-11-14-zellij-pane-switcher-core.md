# Zellij Pane Switcher Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:test-driven-development. Implement each task as a red-green-refactor cycle and keep the worktree runnable after every task.

**Goal:** Build a tested, configurable Zellij session/tab/pane finder launched by a user-defined single keybinding.

**Architecture:** A pure `pane-switcher-core` crate owns world snapshots, filtering, selection, details, and rendering. A thin `pane-switcher-plugin` `[[bin]]` registered with `register_plugin!` maps Zellij events and executes navigation actions.

**Tech Stack:** Rust 2021, `zellij-tile` 0.45.1, `wasm32-wasip1`, `unicode-width`, standard-library tests.

**Spec:** `docs/superpowers/specs/2026-11-14-zellij-pane-switcher-core-design.md`

## Global Constraints

- Target Zellij 0.45.1 and Rust 1.93.0.
- Do not poll commands or CWD; use events and one-time queries.
- Keep `pane-switcher-core` free of Zellij host calls.
- Use stable composite node keys, never row positions.
- Invalid configuration must degrade to defaults without panics.
- Exclude plugin panes and impossible navigation targets.
- Every production behavior is introduced by a failing test first.

---

### Task 1: Workspace and configuration

**Files:**
- Create: `Cargo.toml`
- Create: `crates/pane-switcher-core/Cargo.toml`
- Create: `crates/pane-switcher-core/src/lib.rs`
- Create: `crates/pane-switcher-core/src/config.rs`
- Create: `crates/pane-switcher-core/tests/config_test.rs`

**Interfaces:**
- Produces: `Config`, `DetailsMode`, `IconsMode`, `Density`, `SearchMode`, `SortMode`, and `Config::from_map(&BTreeMap<String, String>)`.

- [ ] **Step 1: Write the failing default and invalid-value test**

```rust
#[test]
fn configuration_defaults_and_invalid_values_are_safe() {
    let config = Config::from_map(&BTreeMap::from([
        ("scope".into(), "nonsense".into()),
        ("show_tabs".into(), "maybe".into()),
        ("details".into(), "sideways".into()),
        ("width".into(), "72%".into()),
    ]));
    assert_eq!(config.scope, Scope::All);
    assert!(config.show_tabs);
    assert_eq!(config.details, DetailsMode::Auto);
    assert_eq!(config.width, "72%");
}
```

- [ ] **Step 2: Run `cargo test -p pane-switcher-core --test config_test` and observe the missing API failure.**
- [ ] **Step 3: Implement the config types and defaulting parser, including booleans, enums, percentage strings, and spinner bounds.**
- [ ] **Step 4: Run the test and the full core test suite.**
- [ ] **Step 5: Commit with `feat: add pane hud configuration model`.**

### Task 2: Stable world snapshots

**Files:**
- Create: `crates/pane-switcher-core/src/model.rs`
- Create: `crates/pane-switcher-core/tests/model_test.rs`
- Modify: `crates/pane-switcher-core/src/lib.rs`

**Interfaces:**
- Produces: `World`, `SessionInput`, `TabInput`, `PaneInput`, `PaneManifestInput`, `DeadSessionInput`, `NodeKey`.
- Produces: `World::apply_snapshot`, `update_current_tabs`, and `update_current_panes`.

- [ ] **Step 1: Write a failing test that applies `TabUpdate` before `PaneUpdate`, then asserts terminal panes stay attached to the current session tab.**
- [ ] **Step 2: Run the focused test and observe the failure.**
- [ ] **Step 3: Implement sorted session/tab/pane storage and stable composite keys; filter plugin and non-selectable panes.**
- [ ] **Step 4: Run focused and full tests.**
- [ ] **Step 5: Commit with `feat: add stable zellij tree model`.**

### Task 3: Fuzzy and substring search

**Files:**
- Create: `crates/pane-switcher-core/src/search.rs`
- Create: `crates/pane-switcher-core/tests/search_test.rs`
- Modify: `crates/pane-switcher-core/src/lib.rs`

**Interfaces:**
- Produces: `MatchScore`, `score_subsequence`, `score_substring`, and `normalize_query`.

- [ ] **Step 1: Write failures for Unicode case-insensitive subsequence matching, contiguity ordering, and non-matches.**
- [ ] **Step 2: Run the focused search test and observe the failure.**
- [ ] **Step 3: Implement a bounded scorer without external fuzzy dependencies.**
- [ ] **Step 4: Run focused and full tests.**
- [ ] **Step 5: Commit with `feat: add cached-friendly tree search scorer`.**

### Task 4: Visible row model and filtering

**Files:**
- Create: `crates/pane-switcher-core/src/rows.rs`
- Create: `crates/pane-switcher-core/tests/rows_test.rs`
- Modify: `crates/pane-switcher-core/src/lib.rs`

**Interfaces:**
- Produces: `Row`, `Rows`, `NodeKind`, and `Rows::build(&World, &Config, &str, &HashSet<NodeKey>, &CommandCache, &CwdCache)`.

- [ ] **Step 1: Write failures for pane-match ancestor preservation, tab-match descendant visibility, session-match tree visibility, and hidden-level flattening.**
- [ ] **Step 2: Run the focused rows test and observe the failure.**
- [ ] **Step 3: Build the complete row tree once per world revision, precompute search text, filter it, and sort siblings by best score then original order.**
- [ ] **Step 4: Add a failing test for stable selection when tree refresh changes row indices; implement selection restoration by `NodeKey`.**
- [ ] **Step 5: Run focused and full tests; commit with `feat: add filtered session tree rows`.**

### Task 5: Input, selection, and icons

**Files:**
- Create: `crates/pane-switcher-core/src/input.rs`
- Create: `crates/pane-switcher-core/src/icons.rs`
- Create: `crates/pane-switcher-core/tests/input_test.rs`
- Create: `crates/pane-switcher-core/tests/icons_test.rs`
- Modify: `crates/pane-switcher-core/src/lib.rs`

**Interfaces:**
- Produces: `TextQuery`, `Selection`, `HudKey`, `HudAction`.
- Produces: `PaneStatus`, `classify_command`, `pane_icon`, and `IconSet`.

- [ ] **Step 1: Write failures for UTF-8 append/backspace, `Ctrl+w`, `Ctrl+u`, wrapped navigation, and collapse/move behavior.**
- [ ] **Step 2: Implement minimal text and selection behavior; run focused tests.**
- [ ] **Step 3: Write failures for focused, exited, shell, codex, Claude, editor, floating, stacked, and unknown icons.**
- [ ] **Step 4: Implement precedence and Nerd/plain icon sets; run focused and full tests.**
- [ ] **Step 5: Commit with `feat: add finder input and pane icons`.**

### Task 6: Details and rendering

**Files:**
- Create: `crates/pane-switcher-core/src/details.rs`
- Create: `crates/pane-switcher-core/src/render.rs`
- Create: `crates/pane-switcher-core/tests/render_test.rs`
- Modify: `crates/pane-switcher-core/src/lib.rs`

**Interfaces:**
- Produces: `Details`, `DetailField`, `LayoutMode`, `RenderFrame`, and `render_frame(...)`.

- [ ] **Step 1: Write failures for full unclipped command/CWD details, unknown placeholders, and config field suppression.**
- [ ] **Step 2: Implement details and run focused tests.**
- [ ] **Step 3: Write failures for narrow-bottom/wide-right layout, tiny-terminal fallback, row viewport, Unicode truncation, and no line exceeding `cols`.**
- [ ] **Step 4: Implement plain-width-safe rendering and ANSI styling; run focused and full tests.**
- [ ] **Step 5: Commit with `feat: render pane hud details and viewport`.**

### Task 7: Zellij adapter and navigation

**Files:**
- Create: `crates/pane-switcher-plugin/Cargo.toml`
- Create: `crates/pane-switcher-plugin/src/main.rs`
- Create: `crates/pane-switcher-plugin/src/adapter.rs`
- Modify: `Cargo.toml`

**Interfaces:**
- Consumes core snapshot, row, input, detail, and render APIs.
- Produces: Zellij plugin registration, event subscriptions, one-time detail queries, and `NavigationTarget`.

- [ ] **Step 1: Add a focused adapter unit test for converting Zellij tab/pane positions into stable core keys and navigation targets.**
- [ ] **Step 2: Observe the adapter test fail, then implement conversion and navigation target selection.**
- [ ] **Step 3: Implement `ZellijPlugin::load`, `update`, `render`, permissions, cached row refresh, timers, cursor placement, and close-after-navigation behavior.**
- [ ] **Step 4: Run host compilation, focused tests, and clippy.**
- [ ] **Step 5: Commit with `feat: wire pane hud into zellij`.**

### Task 8: Packaging, shortcut config, and regression verification

**Files:**
- Create: `examples/config.kdl`
- Create: `scripts/build-wasm.sh`
- Modify: `README.md`
- Modify: `docs/DESIGN.md`

- [ ] **Step 1: Add the default `Ctrl+o`, then `g` session-mode recipe, plus explicit customization instructions.**
- [ ] **Step 2: Add a build script producing both the plugin artifact and absolute paths for local config.**
- [ ] **Step 3: Run `cargo fmt --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and the WASM release build.**
- [ ] **Step 4: Validate the example KDL with the installed Zellij and document any non-interactive limitation separately from unit results.**
- [ ] **Step 5: Commit with `feat: package pane hud plugin and shortcut config`.**
