use std::collections::BTreeMap;

use pane_switcher_core::config::Config;
use pane_switcher_core::model::{LivePane, NodeKey};
use pane_switcher_core::render::{render_frame, RenderOptions};
use pane_switcher_core::rows::{FieldMatches, ProgressState, Row, Rows};
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
        full_dir: dir.into(),
        command: command.into(),
        full_command: command.into(),
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
fn row_fields_use_distinct_semantic_colors() {
    let config = Config::from_map(&BTreeMap::from([(
        "row_format".into(),
        "{title} {dir} {session} {tab} {tab_id} {pane_id} {command}".into(),
    )]));
    let mut shell = row(1, "src renderer", "src/ui", "zsh");
    shell.progress = ProgressState::Idle;
    let rows = Rows { rows: vec![shell] };
    let frame = render_frame(
        &rows,
        None,
        80,
        12,
        "",
        &config,
        0,
        RenderOptions::default(),
    );
    let rendered = &frame.lines[2];

    assert!(rendered.contains("\u{1b}[38;5;51msrc renderer\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;75msrc/ui\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;201mwork\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;208mcode\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;240m11\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;46mzsh\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;240m1\u{1b}[0m"));
}

#[test]
fn selected_rows_keep_field_colors_over_the_selection_background() {
    let config = Config::from_map(&BTreeMap::from([(
        "row_format".into(),
        "{title} {dir} {session} {tab} {tab_id} {pane_id} {command}".into(),
    )]));
    let mut shell = row(1, "src renderer", "src/ui", "zsh");
    shell.progress = ProgressState::Idle;
    let rows = Rows { rows: vec![shell] };
    let frame = render_frame(
        &rows,
        Some(0),
        80,
        12,
        "",
        &config,
        0,
        RenderOptions::default(),
    );
    let rendered = &frame.lines[2];

    assert!(rendered.contains("\u{1b}[38;5;51;48;5;24msrc renderer\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;75;48;5;24msrc/ui\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;201;48;5;24mwork\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;208;48;5;24mcode\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;240;48;5;24m11\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;46;48;5;24mzsh\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;240;48;5;24m1\u{1b}[0m"));
}

#[test]
fn nested_colors_style_template_fragments() {
    let config = Config::from_map(&BTreeMap::from([
        (
            "row_format".into(),
            "{title}  {tab_id}-{pane_id}  {dir}".into(),
        ),
        (
            "colors".into(),
            r#""{title}" "red"
"{tab_id}-{pane_id}" "yellow""#
                .into(),
        ),
    ]));
    let rows = Rows {
        rows: vec![row(1, "src renderer", "src/ui", "codex")],
    };
    let frame = render_frame(
        &rows,
        None,
        80,
        12,
        "",
        &config,
        0,
        RenderOptions::default(),
    );
    let rendered = &frame.lines[2];

    assert!(rendered.contains("\u{1b}[38;5;196msrc renderer\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;226m11-1\u{1b}[0m"));
}

#[test]
fn nested_progress_colors_are_selected_by_state() {
    let config = Config::from_map(&BTreeMap::from([
        ("row_format".into(), "{progress} {title}".into()),
        (
            "colors".into(),
            r#"progress {
    running "green"
    idle "yellow"
    exited "red"
    unknown "magenta"
}"#
            .into(),
        ),
    ]));
    let mut idle = row(2, "idle", ".", "zsh");
    idle.progress = ProgressState::Idle;
    let mut exited = row(3, "exited", ".", "zsh");
    exited.progress = ProgressState::Exited;
    let mut unknown = row(4, "unknown", ".", "?");
    unknown.progress = ProgressState::Unknown;
    let rows = Rows {
        rows: vec![row(1, "running", ".", "codex"), idle, exited, unknown],
    };
    let frame = render_frame(
        &rows,
        None,
        60,
        12,
        "",
        &config,
        0,
        RenderOptions::default(),
    );

    assert!(frame.lines[2].contains("\u{1b}[38;5;46m◐\u{1b}[0m"));
    assert!(frame.lines[3].contains("\u{1b}[38;5;226m✓\u{1b}[0m"));
    assert!(frame.lines[4].contains("\u{1b}[38;5;196m✕\u{1b}[0m"));
    assert!(frame.lines[5].contains("\u{1b}[38;5;201m?\u{1b}[0m"));
}

#[test]
fn idle_progress_uses_a_green_check_mark() {
    let mut idle = row(1, "idle", ".", "zsh");
    idle.progress = ProgressState::Idle;
    let rows = Rows { rows: vec![idle] };
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

    assert!(frame.lines[2].contains("\u{1b}[38;5;46m✓\u{1b}[0m"));
}

#[test]
fn exited_progress_uses_a_red_cross_mark() {
    let mut exited = row(1, "failed", ".", "false");
    exited.progress = ProgressState::Exited;
    let rows = Rows { rows: vec![exited] };
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

    assert!(frame.lines[2].contains("\u{1b}[38;5;196m✕\u{1b}[0m"));
}

#[test]
fn color_rules_prefer_the_longest_match_and_ignore_invalid_values() {
    let config = Config::from_map(&BTreeMap::from([
        ("row_format".into(), "{tab_id}-{pane_id}  {title}".into()),
        (
            "colors".into(),
            r#""{tab_id}" "red"
"{tab_id}-{pane_id}" "yellow"
"{title}" "not-a-color""#
                .into(),
        ),
    ]));
    let rows = Rows {
        rows: vec![row(1, "src renderer", "src/ui", "codex")],
    };
    let frame = render_frame(
        &rows,
        None,
        60,
        12,
        "",
        &config,
        0,
        RenderOptions::default(),
    );
    let rendered = &frame.lines[2];

    assert!(rendered.contains("\u{1b}[38;5;226m11-1\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;51msrc renderer\u{1b}[0m"));
}

#[test]
fn equally_specific_color_rules_use_the_later_definition() {
    let config = Config::from_map(&BTreeMap::from([
        ("row_format".into(), "{title}".into()),
        (
            "colors".into(),
            r#""{title}" "red"
"{title}" "green""#
                .into(),
        ),
    ]));
    let rows = Rows {
        rows: vec![row(1, "src renderer", "src/ui", "codex")],
    };
    let frame = render_frame(
        &rows,
        None,
        60,
        12,
        "",
        &config,
        0,
        RenderOptions::default(),
    );

    assert!(frame.lines[2].contains("\u{1b}[38;5;46msrc renderer\u{1b}[0m"));
}

#[test]
fn shell_type_icons_use_a_visible_green_instead_of_idle_gray() {
    let mut shell = row(1, "shell", ".", "zsh");
    shell.progress = ProgressState::Idle;
    let rows = Rows { rows: vec![shell] };
    let frame = render_frame(
        &rows,
        Some(0),
        80,
        12,
        "",
        &Config::default(),
        0,
        RenderOptions::default(),
    );
    let rendered = frame.lines.join("\n");

    assert!(rendered.contains("38;5;46"));
    assert!(rendered.contains("$"));
    assert!(rendered.contains("✓"));
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

    assert!(frame.lines[2].contains("48;5;24"));
    assert!(frame.lines[2].contains("38;5;51;48;5;24"));
    assert!(frame.lines[2].contains("38;5;201;48;5;24"));
    assert_line_widths(&frame.lines, 60);
}

#[test]
fn detail_panel_text_is_not_rendered() {
    let rendered = render_one(false, "");

    assert!(!rendered.contains("selected\n"));
    assert!(!rendered.contains("command  "));
}

#[test]
fn relies_on_the_zellij_pane_frame_instead_of_drawing_an_inner_border() {
    let rows = Rows {
        rows: vec![row(1, "src renderer", "src/ui", "codex")],
    };
    let frame = render_frame(
        &rows,
        Some(0),
        60,
        12,
        "",
        &Config::default(),
        1,
        RenderOptions {
            color: false,
            ..RenderOptions::default()
        },
    );
    let rendered = frame.lines.join("\n");

    assert!(frame.lines[0].contains("FIND"));
    assert!(!rendered.contains('╭'));
    assert!(!rendered.contains('╰'));
    assert_line_widths(&frame.lines, 60);
}

#[test]
fn footer_documents_tab_navigation() {
    let frame = render_frame(
        &Rows::default(),
        None,
        100,
        12,
        "",
        &Config::default(),
        0,
        RenderOptions {
            color: false,
            ..RenderOptions::default()
        },
    );
    let footer = frame.lines.last().expect("footer");

    assert!(footer.contains("Tab"));
    assert!(footer.contains("Shift+Tab"));
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
        RenderOptions {
            color: false,
            ..RenderOptions::default()
        },
    );

    assert!(frame.lines[2].contains("work/code #1 codex --full-auto [src/ui] src renderer"));
    assert!(!frame.lines[2].contains("fold"));
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
        RenderOptions {
            color: false,
            ..RenderOptions::default()
        },
    );

    assert!(frame.lines[2].contains("src renderer"));
    assert!(frame.lines[2].contains("src/ui"));
    assert!(frame.lines[2].contains("11-1"));
    assert!(frame.lines[2].contains("codex --full-auto"));
}

#[test]
fn default_row_uses_compact_tab_and_pane_id_before_the_title() {
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
        RenderOptions {
            color: false,
            ..RenderOptions::default()
        },
    );

    assert!(frame.lines[2].contains("11-1  src renderer"));
    assert!(!frame.lines[2].contains("work/code"));
    assert!(!frame.lines[2].contains("#1"));
}

#[test]
fn long_command_does_not_hide_the_pane_id() {
    let rows = Rows {
        rows: vec![row(
            1,
            "app",
            ".",
            "codex --full-auto --model gpt-5 --sandbox danger --very-long-argument",
        )],
    };
    let frame = render_frame(
        &rows,
        Some(0),
        80,
        12,
        "",
        &Config::default(),
        1,
        RenderOptions {
            color: false,
            ..RenderOptions::default()
        },
    );

    assert!(frame.lines[2].contains("11-1"));
    assert!(frame.lines[2].contains("codex"));
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
    assert_eq!(frame.cursor.map(|(_, row)| row), Some(0));
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

    assert!(rendered.contains("\u{1b}[38;5;46m"));
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
