use unicode_width::UnicodeWidthStr;

use crate::config::{Config, IconsMode};
use crate::icons::{classify_command, icon_for, PaneVisualState};
use crate::rows::{ProgressState, Row, Rows};
use crate::style::{pad_spans, spans_to_ansi, truncate_spans, ForegroundColor, StyledSpan};
use crate::template::TemplateField;

const BORDER_COLOR: ForegroundColor = ForegroundColor::DarkGray;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderOptions {
    pub color: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self { color: true }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RenderFrame {
    pub lines: Vec<String>,
    pub cursor: Option<(usize, usize)>,
}

#[allow(clippy::too_many_arguments)]
pub fn render_frame(
    rows: &Rows,
    selected: Option<usize>,
    cols: usize,
    height: usize,
    query: &str,
    config: &Config,
    spinner_tick: u64,
    options: RenderOptions,
) -> RenderFrame {
    if cols == 0 || height == 0 {
        return RenderFrame::default();
    }

    let mut lines = Vec::with_capacity(height);
    lines.push(render_text_line(top_border(cols, "FIND"), options));

    let (search, cursor) = search_line(cols, query);
    lines.push(render_spans(search, options));
    lines.push(render_text_line(divider(cols, '├', '┤'), options));

    let row_height = height.saturating_sub(5);
    let selected_index = selected.filter(|index| *index < rows.rows.len());
    let start = selected_index
        .map(|index| (index + 1).saturating_sub(row_height))
        .unwrap_or(0)
        .min(rows.rows.len());
    let end = start.saturating_add(row_height).min(rows.rows.len());
    for (index, row) in rows.rows[start..end].iter().enumerate() {
        let absolute_index = start + index;
        let spans = row_spans(row, config, spinner_tick);
        lines.push(render_spans(
            boxed_spans(spans, cols, selected_index == Some(absolute_index)),
            options,
        ));
    }

    while lines.len() < height.saturating_sub(2) {
        lines.push(render_spans(boxed_spans(Vec::new(), cols, false), options));
    }
    lines.push(render_text_line(
        boxed_text(" ↑↓ move  Enter jump  Esc close ", cols),
        options,
    ));
    lines.push(render_text_line(bottom_border(cols), options));
    lines.truncate(height);

    RenderFrame { lines, cursor }
}

fn row_spans(row: &Row, config: &Config, spinner_tick: u64) -> Vec<StyledSpan> {
    let mut spans = Vec::new();
    for part in config.row_format.parts() {
        match part {
            crate::template::TemplatePart::Literal(text) => {
                push_span(&mut spans, text.clone(), ForegroundColor::Default, false);
            }
            crate::template::TemplatePart::Field(field) => match field {
                TemplateField::Progress => {
                    let (glyph, color) = progress_glyph(row.progress, spinner_tick);
                    push_span(&mut spans, glyph.to_owned(), color, false);
                }
                TemplateField::Icon => {
                    let (glyph, color) = type_icon(row, config.icons);
                    push_span(&mut spans, glyph, color, false);
                }
                TemplateField::Title => {
                    push_matched_text(
                        &mut spans,
                        &row.title,
                        row.matches.get(*field),
                        ForegroundColor::Default,
                    );
                }
                TemplateField::Dir => {
                    push_matched_text(
                        &mut spans,
                        &row.dir,
                        row.matches.get(*field),
                        ForegroundColor::Blue,
                    );
                }
                TemplateField::Session => {
                    push_matched_text(
                        &mut spans,
                        &row.session,
                        row.matches.get(*field),
                        ForegroundColor::Default,
                    );
                }
                TemplateField::Tab => {
                    push_matched_text(
                        &mut spans,
                        &row.tab_name,
                        row.matches.get(*field),
                        ForegroundColor::Default,
                    );
                }
                TemplateField::Command => {
                    push_matched_text(
                        &mut spans,
                        &row.command,
                        row.matches.get(*field),
                        ForegroundColor::Default,
                    );
                }
                TemplateField::PaneId => {
                    push_matched_text(
                        &mut spans,
                        &row.pane.id.to_string(),
                        row.matches.get(*field),
                        ForegroundColor::Default,
                    );
                }
            },
        }
    }
    spans
}

fn progress_glyph(state: ProgressState, spinner_tick: u64) -> (&'static str, ForegroundColor) {
    match state {
        ProgressState::Running => (
            match spinner_tick % 4 {
                0 => "◐",
                1 => "◓",
                2 => "◑",
                _ => "◒",
            },
            ForegroundColor::Cyan,
        ),
        ProgressState::Idle => ("○", ForegroundColor::Gray),
        ProgressState::Exited => ("✖", ForegroundColor::Red),
        ProgressState::Unknown => ("?", ForegroundColor::DarkGray),
    }
}

fn type_icon(row: &Row, mode: IconsMode) -> (String, ForegroundColor) {
    let state = if row.command.trim().is_empty() || row.command.trim() == "?" {
        if row.pane.is_focused {
            PaneVisualState::Focused
        } else if row.pane.is_suppressed {
            PaneVisualState::Stacked
        } else if row.pane.is_fullscreen {
            PaneVisualState::Fullscreen
        } else if row.pane.is_floating {
            PaneVisualState::Floating
        } else {
            PaneVisualState::Unknown
        }
    } else {
        let command = row
            .command
            .split_whitespace()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        classify_command(&command)
    };
    let color = match state {
        PaneVisualState::Focused => ForegroundColor::Green,
        PaneVisualState::Shell => ForegroundColor::Gray,
        PaneVisualState::Command => ForegroundColor::Cyan,
        PaneVisualState::Codex => ForegroundColor::Magenta,
        PaneVisualState::Claude => ForegroundColor::Orange,
        PaneVisualState::Editor => ForegroundColor::Blue,
        PaneVisualState::Floating | PaneVisualState::Fullscreen | PaneVisualState::Stacked => {
            ForegroundColor::Blue
        }
        PaneVisualState::Exited => ForegroundColor::Red,
        PaneVisualState::Unknown => ForegroundColor::DarkGray,
    };
    (icon_for(state, mode).to_owned(), color)
}

fn push_matched_text(
    spans: &mut Vec<StyledSpan>,
    text: &str,
    positions: &[usize],
    foreground: ForegroundColor,
) {
    if positions.is_empty() {
        push_span(spans, text.to_owned(), foreground, false);
        return;
    }

    let mut position_index = 0;
    let mut segment = String::new();
    let mut segment_matched = false;
    for (character_index, character) in text.chars().enumerate() {
        while positions
            .get(position_index)
            .is_some_and(|position| *position < character_index)
        {
            position_index += 1;
        }
        let matched = positions.get(position_index) == Some(&character_index);
        if !segment.is_empty() && matched != segment_matched {
            push_span(
                spans,
                std::mem::take(&mut segment),
                field_segment_color(foreground, segment_matched),
                segment_matched,
            );
        }
        segment_matched = matched;
        segment.push(character);
    }
    if !segment.is_empty() {
        push_span(
            spans,
            segment,
            field_segment_color(foreground, segment_matched),
            segment_matched,
        );
    }
}

fn field_segment_color(base: ForegroundColor, matched: bool) -> ForegroundColor {
    if matched {
        ForegroundColor::Yellow
    } else {
        base
    }
}

fn push_span(spans: &mut Vec<StyledSpan>, text: String, foreground: ForegroundColor, bold: bool) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = spans.last_mut() {
        if last.foreground == foreground && last.bold == bold && !last.selected_background {
            last.text.push_str(&text);
            return;
        }
    }
    spans.push(StyledSpan {
        text,
        foreground,
        selected_background: false,
        bold,
    });
}

fn boxed_spans(spans: Vec<StyledSpan>, cols: usize, selected: bool) -> Vec<StyledSpan> {
    if cols == 0 {
        return Vec::new();
    }
    if cols == 1 {
        let mut spans = truncate_spans(spans, 1);
        if selected {
            set_selected_background(&mut spans);
        }
        return spans;
    }

    let mut inner = truncate_spans(spans, cols - 2);
    inner = pad_spans(inner, cols - 2);
    if selected {
        set_selected_background(&mut inner);
    }

    let mut output = Vec::with_capacity(inner.len() + 2);
    output.push(StyledSpan {
        text: "│".into(),
        foreground: BORDER_COLOR,
        selected_background: selected,
        bold: false,
    });
    output.extend(inner);
    output.push(StyledSpan {
        text: "│".into(),
        foreground: BORDER_COLOR,
        selected_background: selected,
        bold: false,
    });
    output
}

fn set_selected_background(spans: &mut [StyledSpan]) {
    for span in spans {
        span.selected_background = true;
    }
}

fn boxed_text(text: &str, cols: usize) -> Vec<StyledSpan> {
    boxed_spans(
        vec![StyledSpan::new(text, ForegroundColor::Default)],
        cols,
        false,
    )
}

fn search_line(cols: usize, query: &str) -> (Vec<StyledSpan>, Option<(usize, usize)>) {
    if cols < 2 {
        let visible = truncate_text(query, cols);
        return (
            vec![StyledSpan::new(visible, ForegroundColor::Default)],
            None,
        );
    }
    let inner = cols - 2;
    let prefix = " › ";
    let suffix = " Esc ";
    let prefix_width = UnicodeWidthStr::width(prefix);
    let suffix_width = UnicodeWidthStr::width(suffix);
    let available = inner.saturating_sub(prefix_width + suffix_width);
    if available == 0 {
        return (
            boxed_spans(
                vec![StyledSpan::new(
                    truncate_text("search", inner),
                    ForegroundColor::Default,
                )],
                cols,
                false,
            ),
            None,
        );
    }

    let visible_query = tail_by_width(query, available);
    let query_width = UnicodeWidthStr::width(visible_query.as_str());
    let padding = inner.saturating_sub(prefix_width + query_width + suffix_width);
    let content = format!("{prefix}{visible_query}{}{suffix}", " ".repeat(padding));
    let cursor = Some((
        (1 + prefix_width + query_width).min(cols.saturating_sub(2)),
        1,
    ));
    (
        boxed_spans(
            vec![StyledSpan::new(content, ForegroundColor::Default)],
            cols,
            false,
        ),
        cursor,
    )
}

fn top_border(cols: usize, title: &str) -> Vec<StyledSpan> {
    let text = if cols < 2 {
        truncate_text(title, cols)
    } else {
        let inner = cols - 2;
        let title = truncate_text(&format!("─ {title} "), inner);
        let fill = "─".repeat(inner.saturating_sub(UnicodeWidthStr::width(title.as_str())));
        format!("╭{title}{fill}╮")
    };
    vec![StyledSpan::new(text, BORDER_COLOR)]
}

fn bottom_border(cols: usize) -> Vec<StyledSpan> {
    let text = if cols < 2 {
        "─".repeat(cols)
    } else {
        format!("╰{}╯", "─".repeat(cols - 2))
    };
    vec![StyledSpan::new(text, BORDER_COLOR)]
}

fn divider(cols: usize, left: char, right: char) -> Vec<StyledSpan> {
    let text = if cols < 2 {
        "─".repeat(cols)
    } else {
        format!("{left}{}{right}", "─".repeat(cols - 2))
    };
    vec![StyledSpan::new(text, BORDER_COLOR)]
}

fn truncate_text(text: &str, width: usize) -> String {
    spans_to_ansi(
        &truncate_spans(vec![StyledSpan::new(text, ForegroundColor::Default)], width),
        false,
    )
}

fn tail_by_width(text: &str, width: usize) -> String {
    if UnicodeWidthStr::width(text) <= width {
        return text.to_owned();
    }
    let mut tail = Vec::new();
    let mut used = 0;
    for character in text.chars().rev() {
        let char_width = unicode_width::UnicodeWidthChar::width(character).unwrap_or(0);
        if used + char_width > width {
            break;
        }
        tail.push(character);
        used += char_width;
    }
    tail.into_iter().rev().collect()
}

fn render_text_line(spans: Vec<StyledSpan>, options: RenderOptions) -> String {
    render_spans(spans, options)
}

fn render_spans(spans: Vec<StyledSpan>, options: RenderOptions) -> String {
    spans_to_ansi(&spans, options.color)
}
