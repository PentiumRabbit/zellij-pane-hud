use std::borrow::Cow;
use std::path::Path;

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
            TemplateField::PaneId => self.pane_id = positions,
            TemplateField::Progress | TemplateField::Icon => {}
        }
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
    pub command: String,
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
            let mut row = build_row(session, tab, pane, base_cwd, values);
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
    values: &PaneValueCache,
) -> Row {
    let cached_command = values.command(&session.name, pane.id);
    let is_foreground = values.command_is_foreground(&session.name, pane.id);
    let command = cached_command
        .map(|parts| parts.join(" "))
        .or_else(|| pane.initial_command.clone())
        .filter(|command| !command.trim().is_empty())
        .unwrap_or_else(|| "?".to_owned());
    let dir = relative_dir(values.cwd(&session.name, pane.id), base_cwd);

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
        command,
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

fn apply_search(row: &mut Row, config: &Config, normalized_query: &str) -> bool {
    if normalized_query.is_empty() {
        return true;
    }

    let mut best_score: Option<MatchScore> = None;
    for field in config.row_format.search_fields() {
        let text = match field {
            TemplateField::Title => Cow::Borrowed(row.title.as_str()),
            TemplateField::Dir => Cow::Borrowed(row.dir.as_str()),
            TemplateField::Session => Cow::Borrowed(row.session.as_str()),
            TemplateField::Tab => Cow::Borrowed(row.tab_name.as_str()),
            TemplateField::Command => Cow::Borrowed(row.command.as_str()),
            TemplateField::PaneId => Cow::Owned(row.pane.id.to_string()),
            TemplateField::Progress | TemplateField::Icon => continue,
        };
        if let Some(result) = match_field(config.search_mode, normalized_query, &text) {
            best_score = Some(best_score.map_or(result.score, |score| score.max(result.score)));
            row.matches.set(*field, result.positions);
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
