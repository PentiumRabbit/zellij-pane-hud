use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum NodeKey {
    Session {
        session: String,
    },
    Tab {
        session: String,
        tab_id: usize,
    },
    Pane {
        session: String,
        tab_id: usize,
        pane_id: u32,
    },
    DeadSession {
        session: String,
    },
}

impl NodeKey {
    pub fn session_name(&self) -> &str {
        match self {
            Self::Session { session }
            | Self::Tab { session, .. }
            | Self::Pane { session, .. }
            | Self::DeadSession { session } => session,
        }
    }

    pub fn tab_id(&self) -> Option<usize> {
        match self {
            Self::Tab { tab_id, .. } | Self::Pane { tab_id, .. } => Some(*tab_id),
            Self::Session { .. } | Self::DeadSession { .. } => None,
        }
    }

    pub fn pane_id(&self) -> Option<u32> {
        match self {
            Self::Pane { pane_id, .. } => Some(*pane_id),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PaneManifestInput {
    pub panes: BTreeMap<usize, Vec<PaneInput>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaneInput {
    pub id: u32,
    pub title: String,
    pub is_focused: bool,
    pub is_floating: bool,
    pub is_fullscreen: bool,
    pub is_suppressed: bool,
    pub exited: bool,
    pub is_selectable: bool,
    pub is_plugin: bool,
    pub initial_command: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TabInput {
    pub position: usize,
    pub tab_id: usize,
    pub name: String,
    pub active: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionInput {
    pub name: String,
    pub is_current: bool,
    pub connected_clients: usize,
    pub tabs: Vec<TabInput>,
    pub panes: PaneManifestInput,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeadSessionInput {
    pub name: String,
    pub age: Duration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LivePane {
    pub key: NodeKey,
    pub id: u32,
    pub title: String,
    pub is_focused: bool,
    pub is_floating: bool,
    pub is_fullscreen: bool,
    pub is_suppressed: bool,
    pub exited: bool,
    pub initial_command: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiveTab {
    pub position: usize,
    pub tab_id: usize,
    pub name: String,
    pub active: bool,
    pub panes: Vec<LivePane>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiveSession {
    pub name: String,
    pub is_current: bool,
    pub connected_clients: usize,
    pub tabs: Vec<LiveTab>,
}

impl LiveSession {
    pub fn key(&self) -> NodeKey {
        NodeKey::Session {
            session: self.name.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeadSession {
    pub key: NodeKey,
    pub name: String,
    pub age: Duration,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PaneValueCache {
    commands: HashMap<String, HashMap<u32, PaneCommand>>,
    cwds: HashMap<String, HashMap<u32, PathBuf>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaneCommand {
    pub command: Vec<String>,
    pub is_foreground: Option<bool>,
}

impl PaneValueCache {
    pub fn set_command(&mut self, session: &str, pane_id: u32, command: Vec<String>) {
        self.set_command_value(session, pane_id, command, None);
    }

    pub fn set_command_with_foreground(
        &mut self,
        session: &str,
        pane_id: u32,
        command: Vec<String>,
        is_foreground: bool,
    ) {
        self.set_command_value(session, pane_id, command, Some(is_foreground));
    }

    fn set_command_value(
        &mut self,
        session: &str,
        pane_id: u32,
        command: Vec<String>,
        is_foreground: Option<bool>,
    ) {
        self.commands.entry(session.to_owned()).or_default().insert(
            pane_id,
            PaneCommand {
                command,
                is_foreground,
            },
        );
    }

    pub fn command(&self, session: &str, pane_id: u32) -> Option<&[String]> {
        self.commands
            .get(session)
            .and_then(|panes| panes.get(&pane_id))
            .map(|command| command.command.as_slice())
    }

    pub fn command_is_foreground(&self, session: &str, pane_id: u32) -> Option<bool> {
        self.commands
            .get(session)
            .and_then(|panes| panes.get(&pane_id))
            .and_then(|command| command.is_foreground)
    }

    pub fn set_cwd(&mut self, session: &str, pane_id: u32, cwd: PathBuf) {
        self.cwds
            .entry(session.to_owned())
            .or_default()
            .insert(pane_id, cwd);
    }

    pub fn cwd(&self, session: &str, pane_id: u32) -> Option<&std::path::Path> {
        self.cwds
            .get(session)
            .and_then(|panes| panes.get(&pane_id))
            .map(PathBuf::as_path)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct World {
    live_sessions: BTreeMap<String, SessionData>,
    dead_sessions: BTreeMap<String, DeadSession>,
    current_session: Option<String>,
    pending_tabs: Option<Vec<TabInput>>,
    pending_panes: Option<PaneManifestInput>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SessionData {
    tabs: Vec<TabInput>,
    panes_by_position: BTreeMap<usize, Vec<PaneInput>>,
    snapshot: LiveSession,
}

impl World {
    pub fn apply_snapshot(
        &mut self,
        sessions: Vec<SessionInput>,
        dead_sessions: Vec<DeadSessionInput>,
    ) -> bool {
        let mut changed = false;
        let mut live_sessions = BTreeMap::new();
        let mut current_session = None;

        for input in sessions {
            if input.is_current && current_session.is_none() {
                current_session = Some(input.name.clone());
            }
            let panes_by_position = input.panes.panes;
            let mut tabs = input.tabs;
            tabs.sort_by_key(|tab| (tab.position, tab.tab_id));
            let snapshot = build_live_session(
                input.name.clone(),
                input.is_current,
                input.connected_clients,
                &tabs,
                &panes_by_position,
            );
            let data = SessionData {
                tabs,
                panes_by_position,
                snapshot,
            };
            if self.live_sessions.get(&input.name) != Some(&data) {
                changed = true;
            }
            live_sessions.insert(input.name, data);
        }
        if live_sessions.len() != self.live_sessions.len() {
            changed = true;
        }
        if current_session != self.current_session {
            changed = true;
        }
        self.live_sessions = live_sessions;
        self.current_session = current_session;

        let mut dead_by_name = BTreeMap::new();
        for input in dead_sessions {
            let dead_session = DeadSession {
                key: NodeKey::DeadSession {
                    session: input.name.clone(),
                },
                name: input.name.clone(),
                age: input.age,
            };
            if self.dead_sessions.get(&input.name) != Some(&dead_session) {
                changed = true;
            }
            dead_by_name.insert(input.name, dead_session);
        }
        if dead_by_name.len() != self.dead_sessions.len() {
            changed = true;
        }
        self.dead_sessions = dead_by_name;

        if let Some(tabs) = self.pending_tabs.take() {
            changed |= self.set_current_tabs(tabs);
        }
        if let Some(panes) = self.pending_panes.take() {
            changed |= self.set_current_panes(panes);
        }
        changed
    }

    pub fn update_current_tabs(&mut self, mut tabs: Vec<TabInput>) -> bool {
        tabs.sort_by_key(|tab| (tab.position, tab.tab_id));
        if self.current_session.is_none() {
            if self.pending_tabs.as_ref() == Some(&tabs) {
                return false;
            }
            self.pending_tabs = Some(tabs);
            return true;
        }
        self.set_current_tabs(tabs)
    }

    pub fn update_current_panes(&mut self, panes: PaneManifestInput) -> bool {
        if self.current_session.is_none() {
            if self.pending_panes.as_ref() == Some(&panes) {
                return false;
            }
            self.pending_panes = Some(panes);
            return true;
        }
        self.set_current_panes(panes)
    }

    pub fn current_session_name(&self) -> Option<&str> {
        self.current_session.as_deref()
    }

    pub fn live_session(&self, name: &str) -> Option<&LiveSession> {
        self.live_sessions.get(name).map(|data| &data.snapshot)
    }

    pub fn live_sessions(&self) -> impl Iterator<Item = &LiveSession> {
        let mut sessions: Vec<_> = self
            .live_sessions
            .values()
            .map(|data| &data.snapshot)
            .collect();
        sessions.sort_by(|left, right| {
            right
                .is_current
                .cmp(&left.is_current)
                .then_with(|| left.name.cmp(&right.name))
        });
        sessions.into_iter()
    }

    pub fn dead_session(&self, name: &str) -> Option<&DeadSession> {
        self.dead_sessions.get(name)
    }

    pub fn dead_sessions(&self) -> impl Iterator<Item = &DeadSession> {
        self.dead_sessions.values()
    }

    fn set_current_tabs(&mut self, tabs: Vec<TabInput>) -> bool {
        let Some(current) = self.current_session.as_ref() else {
            return false;
        };
        let Some(data) = self.live_sessions.get_mut(current) else {
            return false;
        };
        if data.tabs == tabs {
            return false;
        }
        data.tabs = tabs;
        data.rebuild();
        true
    }

    fn set_current_panes(&mut self, panes: PaneManifestInput) -> bool {
        let Some(current) = self.current_session.as_ref() else {
            return false;
        };
        let Some(data) = self.live_sessions.get_mut(current) else {
            return false;
        };
        if data.panes_by_position == panes.panes {
            return false;
        }
        data.panes_by_position = panes.panes;
        data.rebuild();
        true
    }
}

impl SessionData {
    fn rebuild(&mut self) {
        self.snapshot = build_live_session(
            self.snapshot.name.clone(),
            self.snapshot.is_current,
            self.snapshot.connected_clients,
            &self.tabs,
            &self.panes_by_position,
        );
    }
}

fn build_live_session(
    name: String,
    is_current: bool,
    connected_clients: usize,
    tabs: &[TabInput],
    panes_by_position: &BTreeMap<usize, Vec<PaneInput>>,
) -> LiveSession {
    let tabs = tabs
        .iter()
        .map(|tab| {
            let mut panes = panes_by_position
                .get(&tab.position)
                .into_iter()
                .flatten()
                .filter(|pane| !pane.is_plugin && pane.is_selectable)
                .map(|pane| LivePane {
                    key: NodeKey::Pane {
                        session: name.clone(),
                        tab_id: tab.tab_id,
                        pane_id: pane.id,
                    },
                    id: pane.id,
                    title: pane.title.clone(),
                    is_focused: pane.is_focused,
                    is_floating: pane.is_floating,
                    is_fullscreen: pane.is_fullscreen,
                    is_suppressed: pane.is_suppressed,
                    exited: pane.exited,
                    initial_command: pane.initial_command.clone(),
                })
                .collect::<Vec<_>>();
            panes.sort_by_key(|pane| pane.id);
            LiveTab {
                position: tab.position,
                tab_id: tab.tab_id,
                name: tab.name.clone(),
                active: tab.active,
                panes,
            }
        })
        .collect();

    LiveSession {
        name,
        is_current,
        connected_clients,
        tabs,
    }
}
