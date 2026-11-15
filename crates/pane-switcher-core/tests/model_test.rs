use std::collections::BTreeMap;
use std::time::Duration;

use pane_switcher_core::model::{
    NodeKey, PaneInput, PaneManifestInput, SessionInput, TabInput, World,
};

fn session_without_panes() -> SessionInput {
    SessionInput {
        name: "work".into(),
        is_current: true,
        connected_clients: 1,
        tabs: Vec::new(),
        panes: PaneManifestInput::default(),
    }
}

#[test]
fn tab_update_before_pane_update_preserves_the_stable_pane_key() {
    let mut world = World::default();
    world.apply_snapshot(vec![session_without_panes()], Vec::new());
    world.update_current_tabs(vec![TabInput {
        position: 0,
        tab_id: 42,
        name: "code".into(),
        active: true,
    }]);
    world.update_current_panes(PaneManifestInput {
        panes: BTreeMap::from([(
            0,
            vec![PaneInput {
                id: 7,
                title: "nvim".into(),
                is_focused: true,
                is_floating: false,
                is_fullscreen: false,
                is_suppressed: false,
                exited: false,
                is_selectable: true,
                is_plugin: false,
                initial_command: Some("nvim src/main.rs".into()),
            }],
        )]),
    });

    let pane = world
        .live_session("work")
        .and_then(|session| session.tabs.first())
        .and_then(|tab| tab.panes.first())
        .expect("pane should be materialized");

    assert_eq!(
        pane.key,
        NodeKey::Pane {
            session: "work".into(),
            tab_id: 42,
            pane_id: 7,
        }
    );
    assert_eq!(pane.title, "nvim");
}

#[test]
fn resurrectable_sessions_keep_their_age_and_type() {
    let mut world = World::default();
    world.apply_snapshot(
        Vec::new(),
        vec![pane_switcher_core::model::DeadSessionInput {
            name: "work".into(),
            age: Duration::from_secs(120),
        }],
    );

    assert!(world.live_session("work").is_none());
    assert_eq!(
        world.dead_session("work").map(|dead| dead.age),
        Some(Duration::from_secs(120))
    );
    assert_eq!(
        world.dead_session("work").map(|dead| dead.key.clone()),
        Some(NodeKey::DeadSession {
            session: "work".into(),
        })
    );
}

#[test]
fn identical_world_updates_report_no_change() {
    let tabs = vec![TabInput {
        position: 0,
        tab_id: 42,
        name: "code".into(),
        active: true,
    }];
    let panes = PaneManifestInput {
        panes: BTreeMap::from([(
            0,
            vec![PaneInput {
                id: 7,
                title: "nvim".into(),
                is_focused: true,
                is_floating: false,
                is_fullscreen: false,
                is_suppressed: false,
                exited: false,
                is_selectable: true,
                is_plugin: false,
                initial_command: Some("nvim src/main.rs".into()),
            }],
        )]),
    };
    let session = SessionInput {
        name: "work".into(),
        is_current: true,
        connected_clients: 1,
        tabs: tabs.clone(),
        panes: panes.clone(),
    };
    let mut world = World::default();

    assert!(world.apply_snapshot(vec![session.clone()], Vec::new()));
    assert!(!world.update_current_tabs(tabs));
    assert!(!world.update_current_panes(panes));
    assert!(!world.apply_snapshot(vec![session], Vec::new()));
}

#[test]
fn changed_world_updates_report_change() {
    let tabs = vec![TabInput {
        position: 0,
        tab_id: 42,
        name: "code".into(),
        active: true,
    }];
    let panes = PaneManifestInput {
        panes: BTreeMap::from([(
            0,
            vec![PaneInput {
                id: 7,
                title: "nvim".into(),
                is_focused: true,
                is_floating: false,
                is_fullscreen: false,
                is_suppressed: false,
                exited: false,
                is_selectable: true,
                is_plugin: false,
                initial_command: Some("nvim src/main.rs".into()),
            }],
        )]),
    };
    let mut session = SessionInput {
        name: "work".into(),
        is_current: true,
        connected_clients: 1,
        tabs,
        panes,
    };
    let mut world = World::default();
    assert!(world.apply_snapshot(vec![session.clone()], Vec::new()));

    let mut renamed_tabs = session.tabs.clone();
    renamed_tabs[0].name = "tests".into();
    assert!(world.update_current_tabs(renamed_tabs));

    session.connected_clients = 2;
    assert!(world.apply_snapshot(vec![session], Vec::new()));
}
