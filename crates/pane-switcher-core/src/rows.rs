use std::borrow::Cow;
use std::path::{Path, PathBuf};

use crate::config::{Config, Scope, SearchMode};
use crate::icons::progress_state;
use crate::model::{LivePane, LiveSession, LiveTab, NodeKey, PaneValueCache, World};
use crate::search::{
    match_subsequence_lowercase, match_substring_lowercase, normalize_query, MatchScore,
};
use crate::template::TemplateField;

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
    pub tab_id: Vec<usize>,
    pub pane_id: Vec<usize>,
}

impl FieldMatches {
    pub fn get(&self, field: TemplateField) -> &[usize] {
        match field {
            TemplateField::Session => &self.session,
            TemplateField::Tab => &self.tab,
            TemplateField::Title => &self.title,
            TemplateField::Dir => &self.dir,
            TemplateField::Command => &self.command,
            TemplateField::TabId => &self.tab_id,
            TemplateField::PaneId => &self.pane_id,
            TemplateField::Progress | TemplateField::Icon => &[],
        }
    }

    fn set(&mut self, field: TemplateField, positions: Vec<usize>) {
        match field {
            TemplateField::Session => self.session = positions,
            TemplateField::Tab => self.tab = positions,
            TemplateField::Title => self.title = positions,
            TemplateField::Dir => self.dir = positions,
            TemplateField::Command => self.command = positions,
            TemplateField::TabId => self.tab_id = positions,
            TemplateField::PaneId => self.pane_id = positions,
            TemplateField::Progress | TemplateField::Icon => {}
        }
    }

    fn add_positions(&mut self, field: TemplateField, positions: impl IntoIterator<Item = usize>) {
        let target = match field {
            TemplateField::Session => &mut self.session,
            TemplateField::Tab => &mut self.tab,
            TemplateField::Title => &mut self.title,
            TemplateField::Dir => &mut self.dir,
            TemplateField::Command => &mut self.command,
            TemplateField::TabId => &mut self.tab_id,
            TemplateField::PaneId => &mut self.pane_id,
            TemplateField::Progress | TemplateField::Icon => return,
        };
        target.extend(positions);
        target.sort_unstable();
        target.dedup();
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Row {
    pub key: NodeKey,
    pub session: String,
    pub tab_name: String,
    pub tab_position: usize,
    pub title: String,
    pub dir: String,
    pub full_dir: String,
    pub command: String,
    pub full_command: String,
    pub pane: LivePane,
    pub is_current_session: bool,
    pub progress: ProgressState,
    pub score: u32,
    pub matches: FieldMatches,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Rows {
    pub rows: Vec<Row>,
}

impl Rows {
    pub fn build(world: &World, config: &Config, query: &str, values: &PaneValueCache) -> Self {
        let normalized_query = normalize_query(query);
        let base_cwd = focused_cwd(world, values);
        let mut rows = Vec::new();

        for session in world.live_sessions() {
            if config.scope == Scope::Current && !session.is_current {
                continue;
            }
            append_session_rows(
                &mut rows,
                session,
                base_cwd,
                config,
                values,
                &normalized_query,
            );
        }

        rows.sort_by(|left, right| right.score.cmp(&left.score));
        Self { rows }
    }
}

fn append_session_rows(
    rows: &mut Vec<Row>,
    session: &LiveSession,
    base_cwd: Option<&Path>,
    config: &Config,
    values: &PaneValueCache,
    normalized_query: &str,
) {
    for tab in &session.tabs {
        for pane in &tab.panes {
            let mut row = build_row(session, tab, pane, base_cwd, config, values);
            if apply_search(&mut row, config, normalized_query) {
                rows.push(row);
            }
        }
    }
}

fn build_row(
    session: &LiveSession,
    tab: &LiveTab,
    pane: &LivePane,
    base_cwd: Option<&Path>,
    config: &Config,
    values: &PaneValueCache,
) -> Row {
    let cached_command = values.command(&session.name, pane.id);
    let is_foreground = values.command_is_foreground(&session.name, pane.id);
    let full_command = cached_command
        .map(|parts| parts.join(" "))
        .or_else(|| pane.initial_command.clone())
        .filter(|command| !command.trim().is_empty())
        .unwrap_or_else(|| "?".to_owned());
    let command = abbreviate_command(&full_command, config.command_tail);
    let full_dir = relative_dir(values.cwd(&session.name, pane.id), base_cwd);
    let dir = abbreviate_dir(&full_dir, config.dir_depth);

    Row {
        key: pane.key.clone(),
        session: session.name.clone(),
        tab_name: if tab.name.trim().is_empty() {
            "?".to_owned()
        } else {
            tab.name.clone()
        },
        tab_position: tab.position,
        title: if pane.title.trim().is_empty() {
            format!("pane {}", pane.id)
        } else {
            pane.title.clone()
        },
        dir,
        full_dir,
        command,
        full_command,
        pane: pane.clone(),
        is_current_session: session.is_current,
        progress: progress_state(pane, cached_command, is_foreground),
        score: 0,
        matches: FieldMatches::default(),
    }
}

fn focused_cwd<'a>(world: &World, values: &'a PaneValueCache) -> Option<&'a Path> {
    let current_name = world.current_session_name()?;
    let current = world.live_session(current_name)?;
    current
        .tabs
        .iter()
        .flat_map(|tab| &tab.panes)
        .find(|pane| pane.is_focused)
        .and_then(|pane| values.cwd(current_name, pane.id))
}

pub fn relative_dir(cwd: Option<&Path>, base_cwd: Option<&Path>) -> String {
    match (cwd, base_cwd) {
        (None, _) => "?".to_owned(),
        (Some(path), None) => path.display().to_string(),
        (Some(path), Some(base)) if path == base => ".".to_owned(),
        (Some(path), Some(base)) => path
            .strip_prefix(base)
            .map(|relative| relative.display().to_string())
            .unwrap_or_else(|_| path.display().to_string()),
    }
}

fn abbreviate_dir(dir: &str, depth: usize) -> String {
    if matches!(dir, "." | "?") {
        return dir.to_owned();
    }
    let components = Path::new(dir).components().collect::<Vec<_>>();
    if components.len() <= depth {
        return dir.to_owned();
    }
    let mut abbreviated = PathBuf::new();
    for component in &components[components.len() - depth..] {
        abbreviated.push(component.as_os_str());
    }
    abbreviated.display().to_string()
}

fn abbreviate_command(command: &str, tail: usize) -> String {
    let parts = command.split_whitespace().collect::<Vec<_>>();
    let Some((program, arguments)) = parts.split_first() else {
        return command.to_owned();
    };
    let program = Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(program);
    if arguments.len() <= tail {
        return std::iter::once(program)
            .chain(arguments.iter().copied())
            .collect::<Vec<_>>()
            .join(" ");
    }
    let tail_start = arguments.len() - tail;
    format!("{} … {}", program, arguments[tail_start..].join(" "))
}

fn apply_search(row: &mut Row, config: &Config, normalized_query: &str) -> bool {
    if normalized_query.is_empty() {
        return true;
    }

    let mut best_score: Option<MatchScore> = None;
    for field in config.row_format.search_fields() {
        let text = field_text(row, *field);
        if let Some(result) = match_field(config.search_mode, normalized_query, &text) {
            best_score = Some(best_score.map_or(result.score, |score| score.max(result.score)));
            let display_positions = match_field(
                config.search_mode,
                normalized_query,
                &display_field_text(row, *field),
            )
            .map(|display| display.positions)
            .unwrap_or_default();
            row.matches.set(*field, display_positions);
        }
    }
    if config.search_mode == SearchMode::Fuzzy {
        if let Some(result) = match_combined_fields(row, config, normalized_query, false) {
            let best = result.result.score;
            best_score = Some(best_score.map_or(best, |score| score.max(best)));
            if let Some(display) = match_combined_fields(row, config, normalized_query, true) {
                for (field, positions) in display.positions_by_field {
                    row.matches.add_positions(field, positions);
                }
            }
        }
    }
    match best_score {
        Some(score) => {
            row.score = score.value;
            true
        }
        None => false,
    }
}

struct CombinedMatch {
    result: crate::search::MatchResult,
    positions_by_field: Vec<(TemplateField, Vec<usize>)>,
}

fn match_combined_fields(
    row: &Row,
    config: &Config,
    normalized_query: &str,
    display_values: bool,
) -> Option<CombinedMatch> {
    let fields = config.row_format.search_fields();
    if fields.len() < 2 {
        return None;
    }

    let mut text = String::new();
    let mut segments = Vec::with_capacity(fields.len());
    let mut char_offset = 0;
    for part in config.row_format.parts() {
        match part {
            crate::template::TemplatePart::Literal(literal) => {
                text.push_str(literal);
                char_offset += literal.chars().count();
            }
            crate::template::TemplatePart::Field(field) if field.is_searchable() => {
                let value = if display_values {
                    display_field_text(row, *field)
                } else {
                    field_text(row, *field)
                };
                let start = char_offset;
                let len = value.chars().count();
                text.push_str(&value);
                char_offset += len;
                segments.push((*field, start, len));
            }
            crate::template::TemplatePart::Field(_) => {}
        }
    }

    let normalized_text = text.to_lowercase();
    let result = match_subsequence_lowercase(normalized_query, &normalized_text)?;
    let mut positions_by_field = Vec::<(TemplateField, Vec<usize>)>::new();
    for position in &result.positions {
        let Some((field, start, _)) = segments
            .iter()
            .find(|(_, start, len)| position >= start && position < &(start + len))
        else {
            continue;
        };
        if let Some((_, positions)) = positions_by_field
            .iter_mut()
            .find(|(candidate, _)| candidate == field)
        {
            positions.push(position - start);
        } else {
            positions_by_field.push((*field, vec![position - start]));
        }
    }

    Some(CombinedMatch {
        result,
        positions_by_field,
    })
}

fn field_text<'a>(row: &'a Row, field: TemplateField) -> Cow<'a, str> {
    match field {
        TemplateField::Title => Cow::Borrowed(row.title.as_str()),
        TemplateField::Dir => Cow::Borrowed(row.full_dir.as_str()),
        TemplateField::Session => Cow::Borrowed(row.session.as_str()),
        TemplateField::Tab => Cow::Borrowed(row.tab_name.as_str()),
        TemplateField::Command => Cow::Borrowed(row.full_command.as_str()),
        TemplateField::TabId => Cow::Owned(row.key.tab_id().unwrap_or_default().to_string()),
        TemplateField::PaneId => Cow::Owned(row.pane.id.to_string()),
        TemplateField::Progress | TemplateField::Icon => Cow::Borrowed(""),
    }
}

fn display_field_text<'a>(row: &'a Row, field: TemplateField) -> Cow<'a, str> {
    match field {
        TemplateField::Title => Cow::Borrowed(row.title.as_str()),
        TemplateField::Dir => Cow::Borrowed(row.dir.as_str()),
        TemplateField::Session => Cow::Borrowed(row.session.as_str()),
        TemplateField::Tab => Cow::Borrowed(row.tab_name.as_str()),
        TemplateField::Command => Cow::Borrowed(row.command.as_str()),
        TemplateField::TabId => Cow::Owned(row.key.tab_id().unwrap_or_default().to_string()),
        TemplateField::PaneId => Cow::Owned(row.pane.id.to_string()),
        TemplateField::Progress | TemplateField::Icon => Cow::Borrowed(""),
    }
}

fn match_field(
    mode: SearchMode,
    normalized_query: &str,
    text: &str,
) -> Option<crate::search::MatchResult> {
    let normalized_text = text.to_lowercase();
    match mode {
        SearchMode::Fuzzy => match_subsequence_lowercase(normalized_query, &normalized_text),
        SearchMode::Substring => match_substring_lowercase(normalized_query, &normalized_text),
    }
}
