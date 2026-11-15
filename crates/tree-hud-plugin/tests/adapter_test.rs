use std::collections::HashMap;

use tree_hud_core::app::NavigationTarget;
use tree_hud_core::model::NodeKey;
use tree_hud_plugin::adapter::{
    key_intent, navigation_action, pane_manifest_input, session_input, tab_inputs, InputIntent,
    NavigationAction,
};
use zellij_tile::prelude::{
    BareKey, KeyWithModifier, PaneId, PaneInfo, PaneManifest, SessionInfo, TabInfo,
};

#[test]
fn zellij_snapshot_uses_tab_ids_and_terminal_pane_ids() {
    let session = SessionInfo {
        name: "work".into(),
        tabs: vec![TabInfo {
            position: 2,
            tab_id: 42,
            name: "code".into(),
            active: true,
            ..TabInfo::default()
        }],
        panes: PaneManifest {
            panes: HashMap::from([(
                2,
                vec![PaneInfo {
                    id: 7,
                    title: "nvim".into(),
                    is_focused: true,
                    terminal_command: Some("nvim src/main.rs".into()),
                    is_selectable: true,
                    ..PaneInfo::default()
                }],
            )]),
        },
        is_current_session: true,
        ..SessionInfo::default()
    };

    let input = session_input(&session);
    assert_eq!(input.tabs[0].tab_id, 42);
    assert_eq!(input.panes.panes[&2][0].id, 7);
    assert_eq!(
        NodeKey::Pane {
            session: "work".into(),
            tab_id: input.tabs[0].tab_id,
            pane_id: input.panes.panes[&2][0].id,
        },
        NodeKey::Pane {
            session: "work".into(),
            tab_id: 42,
            pane_id: 7,
        }
    );
}

#[test]
fn adapter_excludes_plugin_and_unselectable_panes() {
    let manifest = PaneManifest {
        panes: HashMap::from([(
            0,
            vec![
                PaneInfo {
                    id: 1,
                    is_plugin: true,
                    is_selectable: true,
                    ..PaneInfo::default()
                },
                PaneInfo {
                    id: 2,
                    is_plugin: false,
                    is_selectable: false,
                    ..PaneInfo::default()
                },
                PaneInfo {
                    id: 3,
                    is_plugin: false,
                    is_selectable: true,
                    ..PaneInfo::default()
                },
            ],
        )]),
    };
    let input = pane_manifest_input(&manifest);
    assert_eq!(input.panes[&0].len(), 3);

    let mut world = tree_hud_core::model::World::default();
    world.apply_snapshot(
        vec![tree_hud_core::model::SessionInput {
            name: "work".into(),
            is_current: true,
            connected_clients: 1,
            tabs: tab_inputs(&[TabInfo {
                position: 0,
                tab_id: 3,
                name: "code".into(),
                active: true,
                ..TabInfo::default()
            }]),
            panes: input,
        }],
        Vec::new(),
    );
    let panes = &world.live_session("work").unwrap().tabs[0].panes;
    assert_eq!(panes.len(), 1);
    assert_eq!(panes[0].id, 3);
}

#[test]
fn navigation_actions_distinguish_current_and_other_sessions() {
    assert_eq!(
        navigation_action(
            &NavigationTarget::FocusPane {
                session: "work".into(),
                tab_position: 2,
                pane_id: 7,
            },
            Some("work"),
        ),
        NavigationAction::FocusCurrentPane {
            pane_id: PaneId::Terminal(7),
        }
    );
    assert_eq!(
        navigation_action(
            &NavigationTarget::FocusPane {
                session: "other".into(),
                tab_position: 2,
                pane_id: 7,
            },
            Some("work"),
        ),
        NavigationAction::SwitchSessionWithPane {
            session: "other".into(),
            tab_position: 2,
            pane_id: 7,
        }
    );
    assert_eq!(
        navigation_action(
            &NavigationTarget::SwitchTab {
                session: "work".into(),
                tab_position: 2,
            },
            Some("work"),
        ),
        NavigationAction::SwitchCurrentTab { tab_position: 2 }
    );
}

#[test]
fn key_adapter_handles_ctrl_shortcuts_and_utf8_text() {
    assert_eq!(
        key_intent(&KeyWithModifier::new(BareKey::Char('p')).with_ctrl_modifier()),
        InputIntent::Up
    );
    assert_eq!(
        key_intent(&KeyWithModifier::new(BareKey::Char('w')).with_ctrl_modifier()),
        InputIntent::DeleteWord
    );
    assert_eq!(
        key_intent(&KeyWithModifier::new(BareKey::Char('u')).with_ctrl_modifier()),
        InputIntent::Clear
    );
    assert_eq!(
        key_intent(&KeyWithModifier::new(BareKey::Char('测'))),
        InputIntent::Insert('测')
    );
    assert_eq!(
        key_intent(&KeyWithModifier::new(BareKey::Esc)),
        InputIntent::Close
    );
    assert_eq!(
        key_intent(&KeyWithModifier::new(BareKey::Left)),
        InputIntent::Ignored
    );
    assert_eq!(
        key_intent(&KeyWithModifier::new(BareKey::Right)),
        InputIntent::Ignored
    );
}
