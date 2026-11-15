use tree_hud_core::style::{pad_spans, spans_to_ansi, truncate_spans, ForegroundColor, StyledSpan};

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
    let spans = pad_spans(vec![span("项目", ForegroundColor::Default)], 6);

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

#[test]
fn colorless_output_contains_only_visible_text() {
    let spans = vec![StyledSpan {
        text: "plain".into(),
        foreground: ForegroundColor::Green,
        selected_background: true,
        bold: true,
    }];

    assert_eq!(spans_to_ansi(&spans, false), "plain");
}

#[test]
fn truncation_preserves_the_original_style_on_the_ellipsis() {
    let truncated = truncate_spans(vec![span("abcdef", ForegroundColor::Cyan)], 4);

    assert_eq!(
        spans_to_ansi(&truncated, true),
        "\u{1b}[38;5;51mabc…\u{1b}[0m"
    );
}
