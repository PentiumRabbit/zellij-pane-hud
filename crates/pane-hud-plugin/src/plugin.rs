use std::collections::BTreeMap;
use std::io::Write;

use pane_hud_core::app::Finder;
use pane_hud_core::config::Config;
use pane_hud_core::render::{render_frame, RenderOptions};
use pane_hud_core::style::ThemePalette;
use pane_hud_core::template::TemplateField;
use zellij_tile::prelude::*;

use crate::adapter::{
    dead_session_inputs, key_intent, navigation_action, pane_manifest_input, session_input,
    tab_inputs, theme_palette, InputIntent, NavigationAction,
};

const MAX_PANE_QUERY_FAILURES: u8 = 2;

#[derive(Default)]
pub struct PaneHudPlugin {
    finder: Finder,
    permissions_granted: bool,
    visible: bool,
    timer_scheduled: bool,
    last_reported_cursor: Option<Option<(usize, usize)>>,
    pane_query_failures: BTreeMap<(String, u32), PaneQueryFailures>,
    theme: ThemePalette,
}

#[derive(Default)]
struct PaneQueryFailures {
    command: u8,
    cwd: u8,
}

impl ZellijPlugin for PaneHudPlugin {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        let config = Config::from_map(&configuration);
        self.finder = Finder::new(config);
        self.visible = true;

        subscribe(&[
            EventType::TabUpdate,
            EventType::PaneUpdate,
            EventType::SessionUpdate,
            EventType::CommandChanged,
            EventType::CwdChanged,
            EventType::Key,
            EventType::Timer,
            EventType::PermissionRequestResult,
            EventType::Visible,
            EventType::BeforeClose,
            EventType::ModeUpdate,
        ]);
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);
        if self.should_report_cursor(None) {
            show_cursor(None);
        }
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PermissionRequestResult(PermissionStatus::Granted) => {
                self.permissions_granted = true;
                self.query_current_session_values();
                self.schedule_spinner();
                true
            }
            Event::PermissionRequestResult(PermissionStatus::Denied) => {
                self.permissions_granted = false;
                true
            }
            Event::SessionUpdate(sessions, resurrectable) => {
                let snapshot = sessions.iter().map(session_input).collect::<Vec<_>>();
                let changed = self
                    .finder
                    .world_mut()
                    .apply_snapshot(snapshot, dead_session_inputs(&resurrectable));
                if changed {
                    self.finder.tree_changed();
                }
                changed | self.query_current_session_values()
            }
            Event::TabUpdate(tabs) => {
                let changed = self
                    .finder
                    .world_mut()
                    .update_current_tabs(tab_inputs(&tabs));
                if changed {
                    self.finder.tree_changed();
                }
                changed
            }
            Event::PaneUpdate(manifest) => {
                let changed = self
                    .finder
                    .world_mut()
                    .update_current_panes(pane_manifest_input(&manifest));
                if changed {
                    self.finder.tree_changed();
                }
                changed | self.query_current_session_values()
            }
            Event::CommandChanged(pane_id, command, is_foreground, _clients) => {
                if let (PaneId::Terminal(pane_id), Some(session)) = (
                    pane_id,
                    self.finder
                        .world()
                        .current_session_name()
                        .map(str::to_owned),
                ) {
                    let changed = self.finder.set_command_with_foreground(
                        &session,
                        pane_id,
                        command,
                        is_foreground,
                    );
                    self.schedule_spinner();
                    changed
                } else {
                    false
                }
            }
            Event::CwdChanged(pane_id, cwd, _clients) => {
                if let (PaneId::Terminal(pane_id), Some(session)) = (
                    pane_id,
                    self.finder
                        .world()
                        .current_session_name()
                        .map(str::to_owned),
                ) {
                    self.finder.set_cwd(&session, pane_id, cwd)
                } else {
                    false
                }
            }
            Event::Key(key) => self.on_key(key),
            Event::Timer(_) => {
                self.timer_scheduled = false;
                if self.visible && self.finder.has_running_visible_pane() {
                    self.finder.advance_spinner();
                    self.arm_spinner();
                    true
                } else {
                    false
                }
            }
            Event::Visible(visible) => {
                let changed = self.visible != visible;
                self.visible = visible;
                if visible {
                    self.query_current_session_values();
                    self.schedule_spinner();
                } else if self.should_report_cursor(None) {
                    show_cursor(None);
                }
                changed
            }
            Event::BeforeClose => {
                if self.should_report_cursor(None) {
                    show_cursor(None);
                }
                false
            }
            Event::ModeUpdate(mode) => {
                self.theme = theme_palette(&mode.style);
                true
            }
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        self.finder.refresh();
        let render_cols = cols.saturating_sub(1).max(1);
        let selected = self.finder.selection().index();
        let query = self.finder.query().to_owned();
        let ticks = self.finder.spinner_tick();
        let frame = render_frame(
            self.finder.rows_ref(),
            Some(selected),
            render_cols,
            rows,
            &query,
            self.finder.config(),
            ticks,
            RenderOptions {
                color: true,
                palette: self.theme,
            },
        );
        if self.should_report_cursor(frame.cursor) {
            show_cursor(frame.cursor);
        }
        let mut stdout = std::io::stdout().lock();
        let _ = write!(stdout, "\u{1b}[2J\u{1b}[H{}", frame.lines.join("\r\n"));
        let _ = stdout.flush();
    }
}

impl PaneHudPlugin {
    fn should_report_cursor(&mut self, cursor: Option<(usize, usize)>) -> bool {
        if self.last_reported_cursor == Some(cursor) {
            return false;
        }
        self.last_reported_cursor = Some(cursor);
        true
    }

    fn on_key(&mut self, key: KeyWithModifier) -> bool {
        match key_intent(&key) {
            InputIntent::Insert(character) => self.finder.insert_char(character),
            InputIntent::Backspace => self.finder.backspace(),
            InputIntent::DeleteWord => self.finder.delete_word(),
            InputIntent::Clear => self.finder.clear_query(),
            InputIntent::Up => self.finder.selection_up(),
            InputIntent::Down => self.finder.selection_down(),
            InputIntent::PageUp => self.finder.page_up(10),
            InputIntent::PageDown => self.finder.page_down(10),
            InputIntent::Enter => {
                self.navigate();
                return true;
            }
            InputIntent::Close => {
                close_self();
                return true;
            }
            InputIntent::Ignored => return false,
        }
        true
    }

    fn navigate(&mut self) {
        let Some(target) = self.finder.navigation_target() else {
            return;
        };
        let action = navigation_action(&target, self.finder.world().current_session_name());
        match action {
            NavigationAction::SwitchSession { session } => switch_session(Some(&session)),
            NavigationAction::SwitchCurrentTab { tab_position } => {
                switch_tab_to((tab_position + 1) as u32)
            }
            NavigationAction::SwitchOtherTab {
                session,
                tab_position,
            } => switch_session_with_focus(&session, Some(tab_position), None),
            NavigationAction::FocusCurrentPane { pane_id } => match pane_id {
                PaneId::Terminal(pane_id) => {
                    focus_terminal_pane(pane_id, true, false);
                }
                PaneId::Plugin(pane_id) => focus_plugin_pane(pane_id, true, false),
            },
            NavigationAction::SwitchSessionWithPane {
                session,
                tab_position,
                pane_id,
            } => switch_session_with_focus(&session, Some(tab_position), Some((pane_id, false))),
        }
        close_self();
    }

    fn query_current_session_values(&mut self) -> bool {
        if !self.permissions_granted {
            return false;
        }
        let Some(session_name) = self
            .finder
            .world()
            .current_session_name()
            .map(str::to_owned)
        else {
            return false;
        };
        let pane_ids = self
            .finder
            .world()
            .live_session(&session_name)
            .map(|session| {
                session
                    .tabs
                    .iter()
                    .flat_map(|tab| tab.panes.iter().map(|pane| pane.id))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        self.update_missing_pane_values(
            &session_name,
            pane_ids,
            |pane_id| get_pane_running_command(PaneId::Terminal(pane_id)).ok(),
            |pane_id| get_pane_cwd(PaneId::Terminal(pane_id)).ok(),
        )
    }

    fn update_missing_pane_values(
        &mut self,
        session_name: &str,
        pane_ids: Vec<u32>,
        mut query_command: impl FnMut(u32) -> Option<Vec<String>>,
        mut query_cwd: impl FnMut(u32) -> Option<std::path::PathBuf>,
    ) -> bool {
        let mut changed = false;
        let template = &self.finder.config().row_format;
        let command_visible = template.contains(TemplateField::Command)
            || template.contains(TemplateField::Progress)
            || template.contains(TemplateField::Icon);
        let cwd_visible = template.contains(TemplateField::Dir);
        for pane_id in pane_ids {
            let failures = self
                .pane_query_failures
                .entry((session_name.to_owned(), pane_id))
                .or_default();
            if command_visible
                && self
                    .finder
                    .values()
                    .command(session_name, pane_id)
                    .is_none()
                && failures.command < MAX_PANE_QUERY_FAILURES
            {
                if let Some(command) = query_command(pane_id) {
                    changed |= self.finder.set_command(session_name, pane_id, command);
                    failures.command = 0;
                } else {
                    failures.command += 1;
                }
            }
            if cwd_visible
                && self.finder.values().cwd(session_name, pane_id).is_none()
                && failures.cwd < MAX_PANE_QUERY_FAILURES
            {
                if let Some(cwd) = query_cwd(pane_id) {
                    changed |= self.finder.set_cwd(session_name, pane_id, cwd);
                    failures.cwd = 0;
                } else {
                    failures.cwd += 1;
                }
            }
        }
        changed
    }

    fn schedule_spinner(&mut self) {
        if self.timer_scheduled
            || !self.visible
            || !self.permissions_granted
            || !self.finder.has_running_visible_pane()
        {
            return;
        }
        self.arm_spinner();
    }

    fn arm_spinner(&mut self) {
        if self.timer_scheduled || !self.visible || !self.permissions_granted {
            return;
        }
        let fps = self.finder.config().spinner_fps.max(1) as f64;
        set_timeout(1.0 / fps);
        self.timer_scheduled = true;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use pane_hud_core::app::Finder;
    use pane_hud_core::config::Config;
    use pane_hud_core::style::ColorSpec;
    use zellij_tile::prelude::{
        BareKey, Event, KeyWithModifier, ModeInfo, PaletteColor, PaneManifest, SessionInfo,
        ZellijPlugin,
    };

    use super::PaneHudPlugin;

    #[test]
    fn cursor_is_reported_only_when_its_position_changes() {
        let mut plugin = PaneHudPlugin::default();

        assert!(plugin.should_report_cursor(None));
        assert!(!plugin.should_report_cursor(None));
        assert!(plugin.should_report_cursor(Some((4, 2))));
        assert!(!plugin.should_report_cursor(Some((4, 2))));
        assert!(plugin.should_report_cursor(Some((5, 2))));
    }

    #[test]
    fn duplicate_structural_events_do_not_request_another_render() {
        let mut plugin = PaneHudPlugin::default();
        let session = SessionInfo {
            name: "work".into(),
            is_current_session: true,
            ..SessionInfo::default()
        };

        assert!(plugin.update(Event::SessionUpdate(vec![session.clone()], Vec::new())));
        plugin.should_report_cursor(Some((4, 2)));

        assert!(!plugin.update(Event::SessionUpdate(vec![session], Vec::new())));
        assert!(!plugin.update(Event::TabUpdate(Vec::new())));
        assert!(!plugin.update(Event::PaneUpdate(PaneManifest {
            panes: HashMap::new(),
        })));
    }

    #[test]
    fn failed_pane_queries_are_retried_until_both_values_are_cached() {
        let mut plugin = PaneHudPlugin::default();

        assert!(!plugin.update_missing_pane_values(
            "work",
            vec![7],
            |_| -> Option<Vec<String>> { None },
            |_| -> Option<PathBuf> { None },
        ));
        assert!(plugin.update_missing_pane_values(
            "work",
            vec![7],
            |_| Some(vec!["cargo".into(), "test".into()]),
            |_| Some(PathBuf::from("/tmp/work")),
        ));
        assert!(!plugin.update_missing_pane_values(
            "work",
            vec![7],
            |_| -> Option<Vec<String>> { panic!("cached command should not be queried") },
            |_| -> Option<PathBuf> { panic!("cached cwd should not be queried") },
        ));
    }

    #[test]
    fn persistently_failing_pane_queries_stop_retrying() {
        let mut plugin = PaneHudPlugin::default();
        let mut command_queries = 0;
        let mut cwd_queries = 0;

        for _ in 0..10 {
            plugin.update_missing_pane_values(
                "work",
                vec![7],
                |_| {
                    command_queries += 1;
                    None
                },
                |_| {
                    cwd_queries += 1;
                    None
                },
            );
        }

        assert_eq!(command_queries, 2);
        assert_eq!(cwd_queries, 2);
    }

    #[test]
    fn hidden_cwd_does_not_trigger_a_host_query() {
        let config = Config::from_map(&std::collections::BTreeMap::from([(
            "row_format".into(),
            "{title}".into(),
        )]));
        let mut plugin = PaneHudPlugin {
            finder: Finder::new(config),
            ..PaneHudPlugin::default()
        };
        let mut command_queries = 0;
        let mut cwd_queries = 0;

        plugin.update_missing_pane_values(
            "work",
            vec![7],
            |_| {
                command_queries += 1;
                Some(vec!["zsh".into()])
            },
            |_| {
                cwd_queries += 1;
                Some(PathBuf::from("/tmp/work"))
            },
        );

        assert_eq!(command_queries, 0);
        assert_eq!(cwd_queries, 0);
    }

    #[test]
    fn left_and_right_keys_do_not_request_render() {
        let mut plugin = PaneHudPlugin::default();

        assert!(!plugin.on_key(KeyWithModifier::new(BareKey::Left)));
        assert!(!plugin.on_key(KeyWithModifier::new(BareKey::Right)));
    }

    #[test]
    fn mode_update_applies_the_active_zellij_theme_palette() {
        let mut plugin = PaneHudPlugin::default();
        let mut mode = ModeInfo::default();
        mode.style.colors.text_unselected.emphasis_2 = PaletteColor::Rgb((1, 2, 3));

        assert!(plugin.update(Event::ModeUpdate(mode)));
        assert_eq!(plugin.theme.green, ColorSpec::Rgb((1, 2, 3)));
    }
}
