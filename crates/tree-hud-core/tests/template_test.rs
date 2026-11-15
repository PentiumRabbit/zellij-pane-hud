use tree_hud_core::template::{RowTemplate, TemplateField, TemplatePart};

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

#[test]
fn templates_report_whether_they_render_a_field() {
    let template = RowTemplate::parse("{progress} {title} {dir}");

    assert!(template.contains(TemplateField::Progress));
    assert!(template.contains(TemplateField::Title));
    assert!(template.contains(TemplateField::Dir));
    assert!(!template.contains(TemplateField::Command));
    assert!(!template.contains(TemplateField::Icon));
}
