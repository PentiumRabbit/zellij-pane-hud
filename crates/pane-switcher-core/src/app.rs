use std::path::PathBuf;

use crate::config::Config;
use crate::input::{Selection, TextQuery};
use crate::model::{NodeKey, PaneValueCache, World};
use crate::rows::{ProgressState, Row, Rows};
use crate::template::TemplateField;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NavigationTarget {
    SwitchSession {
        session: String,
    },
    SwitchTab {
        session: String,
        tab_position: usize,
    },
    FocusPane {
        session: String,
        tab_position: usize,
        pane_id: u32,
    },
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum SelectionIntent {
    #[default]
    Preserve,
    Focused,
    BestMatch,
}

#[derive(Clone, Debug)]
pub struct Finder {
    config: Config,
    world: World,
    values: PaneValueCache,
    query: TextQuery,
    selection: Selection,
    rows: Rows,
    dirty: bool,
    selection_intent: SelectionIntent,
    rebuild_count: u64,
    spinner_tick: u64,
}

impl Default for Finder {
    fn default() -> Self {
        Self::new(Config::default())
    }
}

impl Finder {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            world: World::default(),
            values: PaneValueCache::default(),
            query: TextQuery::default(),
            selection: Selection::default(),
            rows: Rows::default(),
            dirty: true,
            selection_intent: SelectionIntent::Focused,
            rebuild_count: 0,
            spinner_tick: 0,
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn set_snapshot(&mut self, world: World) {
        self.world = world;
        self.tree_changed();
    }

    pub fn tree_changed(&mut self) {
        self.dirty = true;
    }

    pub fn values(&self) -> &PaneValueCache {
        &self.values
    }

    pub fn set_command(&mut self, session: &str, pane_id: u32, command: Vec<String>) -> bool {
        self.set_command_value(session, pane_id, command, None)
    }

    pub fn set_command_with_foreground(
        &mut self,
        session: &str,
        pane_id: u32,
        command: Vec<String>,
        is_foreground: bool,
    ) -> bool {
        self.set_command_value(session, pane_id, command, Some(is_foreground))
    }

    fn set_command_value(
        &mut self,
        session: &str,
        pane_id: u32,
        command: Vec<String>,
        is_foreground: Option<bool>,
    ) -> bool {
        if self.values.command(session, pane_id) == Some(command.as_slice())
            && self.values.command_is_foreground(session, pane_id) == is_foreground
        {
            return false;
        }
        match is_foreground {
            Some(is_foreground) => {
                self.values
                    .set_command_with_foreground(session, pane_id, command, is_foreground);
            }
            None => self.values.set_command(session, pane_id, command),
        }
        if template_uses(
            &self.config,
            &[
                TemplateField::Command,
                TemplateField::Progress,
                TemplateField::Icon,
            ],
        ) {
            self.dirty = true;
        }
        true
    }

    pub fn set_cwd(&mut self, session: &str, pane_id: u32, cwd: PathBuf) -> bool {
        if self.values.cwd(session, pane_id) == Some(cwd.as_path()) {
            return false;
        }
        self.values.set_cwd(session, pane_id, cwd);
        if template_uses(&self.config, &[TemplateField::Dir]) {
            self.dirty = true;
        }
        true
    }

    pub fn query(&self) -> &str {
        self.query.as_str()
    }

    pub fn set_query(&mut self, query: &str) {
        if self.query.as_str() == query {
            return;
        }
        self.query.clear();
        self.query.insert_str(query);
        self.mark_query_changed();
    }

    pub fn insert_char(&mut self, character: char) {
        self.query.insert_char(character);
        self.mark_query_changed();
    }

    pub fn backspace(&mut self) {
        if !self.query.is_empty() {
            self.query.backspace();
            self.mark_query_changed();
        }
    }

    pub fn delete_word(&mut self) {
        if !self.query.is_empty() {
            self.query.delete_word();
            self.mark_query_changed();
        }
    }

    pub fn clear_query(&mut self) {
        if !self.query.is_empty() {
            self.query.clear();
            self.mark_query_changed();
        }
    }

    pub fn rows(&mut self) -> &Rows {
        self.refresh();
        &self.rows
    }

    pub fn rows_ref(&self) -> &Rows {
        &self.rows
    }

    pub fn selection(&self) -> &Selection {
        &self.selection
    }

    pub fn selected_row(&mut self) -> Option<&Row> {
        self.refresh();
        self.rows.rows.get(self.selection.index())
    }

    pub fn select_index(&mut self, index: usize) -> bool {
        self.refresh();
        let Some(row) = self.rows.rows.get(index) else {
            return false;
        };
        let key = row.key.clone();
        self.selection.select_key(&self.rows, &key);
        true
    }

    pub fn refresh(&mut self) {
        if !self.dirty {
            return;
        }
        self.rows = Rows::build(&self.world, &self.config, self.query.as_str(), &self.values);
        let focused = self.focused_key();
        if !self.rows.rows.is_empty() {
            match self.selection_intent {
                SelectionIntent::Preserve => {
                    self.selection.restore(&self.rows, focused.as_ref());
                }
                SelectionIntent::Focused => {
                    if let Some(focused) = focused.as_ref() {
                        self.selection.select_key(&self.rows, focused);
                    } else {
                        self.selection.select_first(&self.rows);
                    }
                    self.selection_intent = SelectionIntent::Preserve;
                }
                SelectionIntent::BestMatch => {
                    self.selection.select_first(&self.rows);
                    self.selection_intent = SelectionIntent::Preserve;
                }
            }
        }
        self.rebuild_count += 1;
        self.dirty = false;
    }

    pub fn selection_up(&mut self) {
        self.refresh();
        self.selection.up(&self.rows);
    }

    pub fn selection_down(&mut self) {
        self.refresh();
        self.selection.down(&self.rows);
    }

    pub fn page_up(&mut self, amount: usize) {
        self.refresh();
        self.selection.page_up(&self.rows, amount);
    }

    pub fn page_down(&mut self, amount: usize) {
        self.refresh();
        self.selection.page_down(&self.rows, amount);
    }

    pub fn navigation_target(&mut self) -> Option<NavigationTarget> {
        self.refresh();
        let row = self.rows.rows.get(self.selection.index())?;
        Some(NavigationTarget::FocusPane {
            session: row.session.clone(),
            tab_position: row.tab_position,
            pane_id: row.pane.id,
        })
    }

    pub fn rebuild_count(&self) -> u64 {
        self.rebuild_count
    }

    pub fn spinner_tick(&self) -> u64 {
        self.spinner_tick
    }

    pub fn advance_spinner(&mut self) {
        self.spinner_tick = (self.spinner_tick + 1) % 4;
    }

    pub fn has_running_visible_pane(&mut self) -> bool {
        self.refresh();
        self.rows
            .rows
            .iter()
            .any(|row| row.is_current_session && row.progress == ProgressState::Running)
    }

    fn focused_key(&self) -> Option<NodeKey> {
        if !self.config.follow_current_session {
            return None;
        }
        let current_name = self.world.current_session_name()?;
        let current = self.world.live_session(current_name)?;
        current
            .tabs
            .iter()
            .find_map(|tab| tab.panes.iter().find(|pane| pane.is_focused))
            .or_else(|| {
                current
                    .tabs
                    .iter()
                    .find(|tab| tab.active)
                    .and_then(|tab| tab.panes.first())
            })
            .or_else(|| current.tabs.iter().flat_map(|tab| &tab.panes).next())
            .map(|pane| pane.key.clone())
    }

    fn mark_query_changed(&mut self) {
        self.dirty = true;
        self.selection_intent = if self.query.as_str().trim().is_empty() {
            SelectionIntent::Focused
        } else {
            SelectionIntent::BestMatch
        };
    }
}

fn template_uses(config: &Config, fields: &[TemplateField]) -> bool {
    fields
        .iter()
        .any(|field| config.row_format.contains(*field))
}
