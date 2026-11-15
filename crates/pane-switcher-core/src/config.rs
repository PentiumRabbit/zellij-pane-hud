use std::collections::BTreeMap;

use crate::template::RowTemplate;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Scope {
    #[default]
    All,
    Current,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DetailsMode {
    #[default]
    Auto,
    Bottom,
    Right,
    Off,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum IconsMode {
    #[default]
    Nerd,
    Plain,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Density {
    #[default]
    Compact,
    Comfortable,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SearchMode {
    #[default]
    Fuzzy,
    Substring,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SortMode {
    #[default]
    SessionTabPane,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub scope: Scope,
    pub show_sessions: bool,
    pub show_tabs: bool,
    pub show_panes: bool,
    pub include_resurrectable: bool,
    pub show_commands: bool,
    pub show_cwd: bool,
    pub details: DetailsMode,
    pub detail_status: bool,
    pub detail_cwd: bool,
    pub detail_command: bool,
    pub show_pane_ids: bool,
    pub follow_current_session: bool,
    pub sort: SortMode,
    pub icons: IconsMode,
    pub density: Density,
    pub width: String,
    pub height: String,
    pub search_mode: SearchMode,
    pub spinner_fps: u8,
    pub row_format: RowTemplate,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scope: Scope::All,
            show_sessions: true,
            show_tabs: true,
            show_panes: true,
            include_resurrectable: true,
            show_commands: true,
            show_cwd: true,
            details: DetailsMode::Auto,
            detail_status: true,
            detail_cwd: true,
            detail_command: true,
            show_pane_ids: false,
            follow_current_session: true,
            sort: SortMode::SessionTabPane,
            icons: IconsMode::Nerd,
            density: Density::Compact,
            width: "72%".to_owned(),
            height: "68%".to_owned(),
            search_mode: SearchMode::Fuzzy,
            spinner_fps: 4,
            row_format: RowTemplate::default(),
        }
    }
}

impl Config {
    pub fn from_map(values: &BTreeMap<String, String>) -> Self {
        let mut config = Self::default();

        set_enum(&mut config.scope, values, "scope", parse_scope);
        set_bool(&mut config.show_sessions, values, "show_sessions");
        set_bool(&mut config.show_tabs, values, "show_tabs");
        set_bool(&mut config.show_panes, values, "show_panes");
        set_bool(
            &mut config.include_resurrectable,
            values,
            "include_resurrectable",
        );
        set_bool(&mut config.show_commands, values, "show_commands");
        set_bool(&mut config.show_cwd, values, "show_cwd");
        set_enum(&mut config.details, values, "details", parse_details);
        set_bool(&mut config.detail_status, values, "detail_status");
        set_bool(&mut config.detail_cwd, values, "detail_cwd");
        set_bool(&mut config.detail_command, values, "detail_command");
        set_bool(&mut config.show_pane_ids, values, "show_pane_ids");
        set_bool(
            &mut config.follow_current_session,
            values,
            "follow_current_session",
        );
        set_enum(&mut config.sort, values, "sort", parse_sort);
        set_enum(&mut config.icons, values, "icons", parse_icons);
        set_enum(&mut config.density, values, "density", parse_density);
        set_enum(
            &mut config.search_mode,
            values,
            "search_mode",
            parse_search_mode,
        );

        if let Some(width) = values.get("width").and_then(|value| parse_size(value)) {
            config.width = width;
        }
        if let Some(height) = values.get("height").and_then(|value| parse_size(value)) {
            config.height = height;
        }
        if let Some(fps) = values
            .get("spinner_fps")
            .and_then(|value| value.trim().parse::<u8>().ok())
        {
            config.spinner_fps = fps.clamp(1, 30);
        }
        if let Some(format) = values.get("row_format") {
            if !format.trim().is_empty() {
                config.row_format = RowTemplate::parse(format);
            }
        }

        config
    }
}

fn set_bool(target: &mut bool, values: &BTreeMap<String, String>, key: &str) {
    if let Some(value) = values.get(key).and_then(|value| parse_bool(value)) {
        *target = value;
    }
}

fn set_enum<T>(
    target: &mut T,
    values: &BTreeMap<String, String>,
    key: &str,
    parser: impl Fn(&str) -> Option<T>,
) {
    if let Some(value) = values.get(key).and_then(|value| parser(value)) {
        *target = value;
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn parse_scope(value: &str) -> Option<Scope> {
    match value.trim().to_ascii_lowercase().as_str() {
        "all" => Some(Scope::All),
        "current" => Some(Scope::Current),
        _ => None,
    }
}

fn parse_details(value: &str) -> Option<DetailsMode> {
    match value.trim().to_ascii_lowercase().as_str() {
        "auto" => Some(DetailsMode::Auto),
        "bottom" => Some(DetailsMode::Bottom),
        "right" => Some(DetailsMode::Right),
        "off" => Some(DetailsMode::Off),
        _ => None,
    }
}

fn parse_icons(value: &str) -> Option<IconsMode> {
    match value.trim().to_ascii_lowercase().as_str() {
        "nerd" => Some(IconsMode::Nerd),
        "plain" => Some(IconsMode::Plain),
        _ => None,
    }
}

fn parse_density(value: &str) -> Option<Density> {
    match value.trim().to_ascii_lowercase().as_str() {
        "compact" => Some(Density::Compact),
        "comfortable" => Some(Density::Comfortable),
        _ => None,
    }
}

fn parse_search_mode(value: &str) -> Option<SearchMode> {
    match value.trim().to_ascii_lowercase().as_str() {
        "fuzzy" => Some(SearchMode::Fuzzy),
        "substring" => Some(SearchMode::Substring),
        _ => None,
    }
}

fn parse_sort(value: &str) -> Option<SortMode> {
    match value.trim().to_ascii_lowercase().as_str() {
        "session-tab-pane" => Some(SortMode::SessionTabPane),
        _ => None,
    }
}

fn parse_size(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if let Some(percent) = trimmed.strip_suffix('%') {
        let percent = percent.parse::<u16>().ok()?;
        if (1..=100).contains(&percent) {
            return Some(format!("{percent}%"));
        }
        return None;
    }
    let fixed = trimmed.parse::<u16>().ok()?;
    (fixed > 0).then(|| fixed.to_string())
}
