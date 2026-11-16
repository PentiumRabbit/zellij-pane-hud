use std::collections::BTreeMap;

use pane_switcher_core::config::{Config, DetailsMode, Scope};
use pane_switcher_core::template::{RowTemplate, TemplateField};

#[test]
fn configuration_parses_values_and_falls_back_per_field() {
    let config = Config::from_map(&BTreeMap::from([
        ("scope".into(), "current".into()),
        ("show_tabs".into(), "false".into()),
        ("details".into(), "sideways".into()),
        ("width".into(), "72%".into()),
    ]));

    assert_eq!(config.scope, Scope::Current);
    assert!(!config.show_tabs);
    assert_eq!(config.details, DetailsMode::Auto);
    assert_eq!(config.width, "72%");
}

#[test]
fn pane_size_defaults_to_percentages_and_accepts_fixed_dimensions() {
    let default = Config::default();
    assert_eq!(default.width, "72%");
    assert_eq!(default.height, "68%");

    let fixed = Config::from_map(&BTreeMap::from([
        ("width".into(), "120".into()),
        ("height".into(), "40".into()),
    ]));
    assert_eq!(fixed.width, "120");
    assert_eq!(fixed.height, "40");

    let invalid = Config::from_map(&BTreeMap::from([
        ("width".into(), "0".into()),
        ("height".into(), "101%".into()),
    ]));
    assert_eq!(invalid.width, "72%");
    assert_eq!(invalid.height, "68%");
}

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
