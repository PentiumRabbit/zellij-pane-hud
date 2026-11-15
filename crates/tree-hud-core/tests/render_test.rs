use std::collections::BTreeMap;

use tree_hud_core::config::Config;
use tree_hud_core::model::{LivePane, NodeKey};
use tree_hud_core::render::{render_frame, RenderOptions};
use tree_hud_core::rows::{FieldMatches, ProgressState, Row, Rows};
use unicode_width::UnicodeWidthStr;

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
        matches: FieldMatches::default(),
    }
}

fn render_one(selected: bool, query: &str) -> String {
    let config = Config::default();
    let mut first = row(1, "src renderer", "src/ui", "codex --full-auto");
    if !query.is_empty() {
        first.matches.title = vec![0];
    }
    let panes = Rows { rows: vec![first] };
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
    let config = Config::default();
    let rows = Rows {
        rows: vec![row(1, "src renderer", "src/ui", "codex --full-auto")],
    };
    let frame = render_frame(
        &rows,
        Some(0),
        60,
        12,
        "",
        &config,
        1,
        RenderOptions::default(),
    );

    assert!(frame.lines[3].contains("48;5;24"));
    assert!(frame.lines[3].contains("38;5;51;48;5;24"));
    assert!(frame.lines[3].contains("38;5;201;48;5;24"));
    assert_line_widths(&frame.lines, 60);
}

#[test]
fn detail_panel_text_is_not_rendered() {
    let rendered = render_one(false, "");

    assert!(!rendered.contains("selected\n"));
    assert!(!rendered.contains("command  "));
}

#[test]
fn custom_template_controls_field_order_and_visible_values() {
    let config = Config::from_map(&BTreeMap::from([(
        "row_format".into(),
        "{icon} {session}/{tab} #{pane_id} {command} [{dir}] {title}".into(),
    )]));
    let rows = Rows {
        rows: vec![row(1, "src renderer", "src/ui", "codex --full-auto")],
    };
    let frame = render_frame(
        &rows,
        Some(0),
        80,
        12,
        "",
        &config,
        1,
        RenderOptions { color: false },
    );

    assert!(frame.lines[3].contains("work/code #1 codex --full-auto [src/ui] src renderer"));
    assert!(!frame.lines[3].contains("fold"));
}

#[test]
fn default_row_shows_all_pane_information() {
    let rows = Rows {
        rows: vec![row(1, "src renderer", "src/ui", "codex --full-auto")],
    };
    let frame = render_frame(
        &rows,
        Some(0),
        120,
        12,
        "",
        &Config::default(),
        1,
        RenderOptions { color: false },
    );

    assert!(frame.lines[3].contains("src renderer"));
    assert!(frame.lines[3].contains("src/ui"));
    assert!(frame.lines[3].contains("work/code"));
    assert!(frame.lines[3].contains("codex --full-auto"));
    assert!(frame.lines[3].contains("#1"));
}

#[test]
fn every_line_is_unicode_width_safe_and_the_frame_is_exact_height() {
    let config = Config::default();
    let rows = Rows {
        rows: vec![row(
            1,
            "很长的项目标题abcdefghijklmnopqrstuvwxyz",
            "很长的目录abcdefghijklmnopqrstuvwxyz",
            "codex",
        )],
    };
    let frame = render_frame(
        &rows,
        Some(0),
        24,
        9,
        "很长的查询abcdefghijklmnopqrstuvwxyz",
        &config,
        0,
        RenderOptions::default(),
    );

    assert_eq!(frame.lines.len(), 9);
    assert_line_widths(&frame.lines, 24);
    assert_eq!(frame.cursor.map(|(_, row)| row), Some(1));
}

#[test]
fn viewport_rendering_stays_bounded_for_ten_thousand_rows() {
    let config = Config::default();
    let rows = Rows {
        rows: (1..=10_000)
            .map(|id| row(id, &format!("pane-{id}"), ".", "zsh"))
            .collect(),
    };
    let frame = render_frame(
        &rows,
        Some(9_999),
        80,
        10,
        "",
        &config,
        0,
        RenderOptions::default(),
    );
    let rendered = frame.lines.join("\n");

    assert!(rendered.contains("pane-10000"));
    assert!(!rendered.contains("pane-1 "));
    assert!(!rendered.contains("pane-5000"));
    assert_line_widths(&frame.lines, 80);
}

#[test]
fn progress_states_have_distinct_semantic_colors() {
    let mut idle = row(1, "idle", ".", "zsh");
    idle.progress = ProgressState::Idle;
    let mut exited = row(2, "exited", ".", "vim");
    exited.progress = ProgressState::Exited;
    let mut unknown = row(3, "unknown", ".", "?");
    unknown.progress = ProgressState::Unknown;
    let rows = Rows {
        rows: vec![idle, exited, unknown],
    };
    let frame = render_frame(
        &rows,
        None,
        80,
        12,
        "",
        &Config::default(),
        0,
        RenderOptions::default(),
    );
    let rendered = frame.lines.join("\n");

    assert!(rendered.contains("\u{1b}[38;5;245m"));
    assert!(rendered.contains("\u{1b}[38;5;196m"));
    assert!(rendered.contains("\u{1b}[38;5;240m"));
}

#[test]
fn one_cell_terminals_still_receive_width_safe_output() {
    let rows = Rows {
        rows: vec![row(1, "wide 项目", "src", "zsh")],
    };
    let frame = render_frame(
        &rows,
        Some(0),
        1,
        1,
        "query",
        &Config::default(),
        0,
        RenderOptions::default(),
    );

    assert_eq!(frame.lines.len(), 1);
    assert_line_widths(&frame.lines, 1);
}

fn assert_line_widths(lines: &[String], expected: usize) {
    for line in lines {
        let visible = strip_sgr(line);
        assert_eq!(UnicodeWidthStr::width(visible.as_str()), expected);
    }
}

fn strip_sgr(text: &str) -> String {
    let mut output = String::new();
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if next == 'm' {
                    break;
                }
            }
        } else {
            output.push(character);
        }
    }
    output
}
