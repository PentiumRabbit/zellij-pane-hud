use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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

impl ForegroundColor {
    fn ansi_value(self) -> u8 {
        match self {
            Self::Default => 252,
            Self::Gray => 245,
            Self::Green => 46,
            Self::Cyan => 51,
            Self::Magenta => 201,
            Self::Orange => 208,
            Self::Blue => 75,
            Self::Red => 196,
            Self::Yellow => 226,
            Self::DarkGray => 240,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StyledSpan {
    pub text: String,
    pub foreground: ForegroundColor,
    pub selected_background: bool,
    pub bold: bool,
}

impl StyledSpan {
    pub fn new(text: impl Into<String>, foreground: ForegroundColor) -> Self {
        Self {
            text: text.into(),
            foreground,
            selected_background: false,
            bold: false,
        }
    }
}

pub fn truncate_spans(spans: Vec<StyledSpan>, width: usize) -> Vec<StyledSpan> {
    if width == 0 {
        return Vec::new();
    }
    let total_width = spans
        .iter()
        .map(|span| UnicodeWidthStr::width(span.text.as_str()))
        .sum::<usize>();
    if total_width <= width {
        return spans;
    }

    let content_width = width.saturating_sub(1);
    let mut output = Vec::new();
    let mut used = 0;

    for span in spans {
        let mut text = String::new();
        for character in span.text.chars() {
            let character_width = UnicodeWidthChar::width(character).unwrap_or(0);
            if used + character_width > content_width {
                push_nonempty(&mut output, span_style_with_text(&span, text));
                push_nonempty(
                    &mut output,
                    StyledSpan {
                        text: "…".to_owned(),
                        ..span
                    },
                );
                return output;
            }
            text.push(character);
            used += character_width;
        }
        push_nonempty(&mut output, span_style_with_text(&span, text));
    }

    if let Some(style) = output.last().cloned() {
        push_nonempty(
            &mut output,
            StyledSpan {
                text: "…".to_owned(),
                ..style
            },
        );
    }
    output
}

pub fn pad_spans(mut spans: Vec<StyledSpan>, width: usize) -> Vec<StyledSpan> {
    let visible = spans
        .iter()
        .map(|span| UnicodeWidthStr::width(span.text.as_str()))
        .sum::<usize>();
    if visible < width {
        spans.push(StyledSpan::new(
            " ".repeat(width - visible),
            ForegroundColor::Default,
        ));
    }
    spans
}

pub fn spans_to_ansi(spans: &[StyledSpan], color: bool) -> String {
    if !color {
        return spans.iter().map(|span| span.text.as_str()).collect();
    }

    let mut output = String::new();
    for span in spans {
        let styled =
            span.foreground != ForegroundColor::Default || span.selected_background || span.bold;
        if !styled {
            output.push_str(&span.text);
            continue;
        }

        output.push_str("\u{1b}[");
        let mut first = true;
        if span.bold {
            output.push('1');
            first = false;
        }
        if !first {
            output.push(';');
        }
        output.push_str("38;5;");
        output.push_str(&span.foreground.ansi_value().to_string());
        if span.selected_background {
            output.push_str(";48;5;24");
        }
        output.push('m');
        output.push_str(&span.text);
        output.push_str("\u{1b}[0m");
    }
    output
}

fn span_style_with_text(span: &StyledSpan, text: String) -> StyledSpan {
    StyledSpan {
        text,
        ..span.clone()
    }
}

fn push_nonempty(output: &mut Vec<StyledSpan>, span: StyledSpan) {
    if !span.text.is_empty() {
        if let Some(last) = output.last_mut() {
            if last.foreground == span.foreground
                && last.selected_background == span.selected_background
                && last.bold == span.bold
            {
                last.text.push_str(&span.text);
                return;
            }
        }
        output.push(span);
    }
}
