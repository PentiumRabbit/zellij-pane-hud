use std::collections::BTreeMap;
use std::path::PathBuf;

use pane_switcher_core::app::{Finder, NavigationTarget};
use pane_switcher_core::config::Config;
use pane_switcher_core::model::{
    NodeKey, PaneInput, PaneManifestInput, SessionInput, TabInput, World,
};

fn world_with(panes: Vec<PaneInput>) -> World {
    let mut world = World::default();
    world.apply_snapshot(
        vec![SessionInput {
            name: "work".into(),
            is_current: true,
            connected_clients: 1,
            tabs: vec![TabInput {
                position: 0,
                tab_id: 42,
                name: "code".into(),
                active: true,
            }],
            panes: PaneManifestInput {
                panes: BTreeMap::from([(0, panes)]),
            },
        }],
        Vec::new(),
    );
    world
}

fn pane(id: u32, focused: bool) -> PaneInput {
    PaneInput {
        id,
        title: format!("pane-{id}"),
        is_focused: focused,
        is_floating: false,
        is_fullscreen: false,
        is_suppressed: false,
        exited: false,
        is_selectable: true,
        is_plugin: false,
        initial_command: Some("zsh".into()),
    }
}

#[test]
fn finder_initially_selects_the_focused_pane() {
    let mut finder = Finder::new(Config::default());
    finder.set_snapshot(world_with(vec![pane(1, false), pane(2, true)]));
    finder.refresh();

    assert_eq!(
        finder.selection().selected_key(),
        Some(&NodeKey::Pane {
            session: "work".into(),
            tab_id: 42,
            pane_id: 2,
        })
    );
}

#[test]
fn query_change_selects_the_highest_scored_match() {
    let mut focused = pane(1, true);
    focused.title = "codex-api".into();
    let mut shorter_match = pane(2, false);
    shorter_match.title = "code".into();
    let mut finder = Finder::new(Config::default());
    finder.set_snapshot(world_with(vec![focused, shorter_match]));
    finder.refresh();

    finder.set_query("code");

    assert_eq!(
        finder.selected_row().map(|row| row.title.as_str()),
        Some("code")
    );
}

#[test]
fn typing_selects_the_highest_scored_match() {
    let mut focused = pane(1, true);
    focused.title = "codex-api".into();
    let mut shorter_match = pane(2, false);
    shorter_match.title = "code".into();
    let mut finder = Finder::new(Config::default());
    finder.set_snapshot(world_with(vec![focused, shorter_match]));
    finder.refresh();

    finder.insert_char('c');

    assert_eq!(
        finder.selected_row().map(|row| row.title.as_str()),
        Some("code")
    );
}

#[test]
fn backspace_selects_the_highest_scored_match() {
    let mut focused = pane(1, true);
    focused.title = "codex-api".into();
    let mut shorter_match = pane(2, false);
    shorter_match.title = "code".into();
    let mut finder = Finder::new(Config::default());
    finder.set_snapshot(world_with(vec![focused, shorter_match]));
    finder.refresh();
    finder.set_query("cod");
    finder.selection_down();

    finder.backspace();

    assert_eq!(
        finder.selected_row().map(|row| row.title.as_str()),
        Some("code")
    );
}

#[test]
fn delete_word_selects_the_highest_scored_match() {
    let mut focused = pane(1, true);
    focused.title = "codex-api extra".into();
    let mut shorter_match = pane(2, false);
    shorter_match.title = "code extra".into();
    let mut finder = Finder::new(Config::default());
    finder.set_snapshot(world_with(vec![focused, shorter_match]));
    finder.refresh();
    finder.set_query("cod extra");
    finder.selection_down();

    finder.delete_word();

    assert_eq!(
        finder.selected_row().map(|row| row.title.as_str()),
        Some("code extra")
    );
}

#[test]
fn clearing_the_query_restores_the_focused_pane() {
    let mut shorter_match = pane(1, false);
    shorter_match.title = "code".into();
    let mut focused = pane(2, true);
    focused.title = "codex-api".into();
    let mut finder = Finder::new(Config::default());
    finder.set_snapshot(world_with(vec![shorter_match, focused]));
    finder.refresh();
    finder.set_query("code");
    assert_eq!(finder.selected_row().map(|row| row.pane.id), Some(1));

    finder.clear_query();

    assert_eq!(finder.selected_row().map(|row| row.pane.id), Some(2));
}

#[test]
fn whitespace_only_query_keeps_the_focused_pane_selected() {
    let mut shorter_match = pane(1, false);
    shorter_match.title = "code".into();
    let mut focused = pane(2, true);
    focused.title = "codex-api".into();
    let mut finder = Finder::new(Config::default());
    finder.set_snapshot(world_with(vec![shorter_match, focused]));
    finder.refresh();

    finder.set_query("   ");

    assert_eq!(finder.selected_row().map(|row| row.pane.id), Some(2));
}

#[test]
fn data_refresh_with_an_unchanged_query_preserves_manual_selection() {
    let mut shorter_match = pane(1, false);
    shorter_match.title = "code".into();
    let mut focused = pane(2, true);
    focused.title = "codex-api".into();
    let mut finder = Finder::new(Config::default());
    finder.set_snapshot(world_with(vec![shorter_match, focused]));
    finder.refresh();
    finder.set_query("code");
    finder.selection_down();
    assert_eq!(finder.selected_row().map(|row| row.pane.id), Some(2));

    finder.set_command("work", 1, vec!["cargo".into(), "test".into()]);

    assert_eq!(finder.selected_row().map(|row| row.pane.id), Some(2));
}

#[test]
fn query_typed_before_rows_arrive_selects_the_highest_scored_match() {
    let mut focused = pane(1, true);
    focused.title = "codex-api".into();
    let mut shorter_match = pane(2, false);
    shorter_match.title = "code".into();
    let mut finder = Finder::new(Config::default());
    finder.set_query("code");
    finder.refresh();

    finder.set_snapshot(world_with(vec![focused, shorter_match]));

    assert_eq!(
        finder.selected_row().map(|row| row.title.as_str()),
        Some("code")
    );
}

#[test]
fn flattened_pane_navigation_keeps_session_tab_and_pane_identity() {
    let mut finder = Finder::new(Config::default());
    finder.set_snapshot(world_with(vec![pane(7, true)]));
    finder.refresh();

    assert_eq!(
        finder.navigation_target(),
        Some(NavigationTarget::FocusPane {
            session: "work".into(),
            tab_position: 0,
            pane_id: 7,
        })
    );
}

#[test]
fn repeated_render_and_selection_do_not_rebuild_search_rows() {
    let panes = (1..=2_000)
        .map(|id| pane(id, id == 1_000))
        .collect::<Vec<_>>();
    let mut finder = Finder::new(Config::default());
    finder.set_snapshot(world_with(panes));
    finder.refresh();
    let rebuilds = finder.rebuild_count();

    for _ in 0..50 {
        finder.refresh();
        finder.selection_down();
        finder.selection_up();
    }
    assert_eq!(finder.rebuild_count(), rebuilds);

    finder.set_query("pane-1999");
    assert_eq!(finder.rebuild_count(), rebuilds);
    finder.refresh();
    assert_eq!(finder.rebuild_count(), rebuilds + 1);
}

#[test]
fn idle_shell_does_not_keep_the_spinner_running() {
    let mut finder = Finder::new(Config::default());
    finder.set_snapshot(world_with(vec![pane(1, true)]));
    finder.set_command("work", 1, vec!["/bin/zsh".into()]);

    assert!(!finder.has_running_visible_pane());
}

#[test]
fn duplicate_pane_value_updates_report_no_change() {
    let mut finder = Finder::new(Config::default());
    finder.set_snapshot(world_with(vec![pane(1, true)]));
    finder.refresh();

    assert!(finder.set_command("work", 1, vec!["cargo".into(), "test".into()]));
    assert!(!finder.set_command("work", 1, vec!["cargo".into(), "test".into()]));
    assert!(finder.set_cwd("work", 1, PathBuf::from("/tmp/work")));
    assert!(!finder.set_cwd("work", 1, PathBuf::from("/tmp/work")));
}

#[test]
fn deferred_initial_world_still_selects_the_focused_pane() {
    let mut finder = Finder::new(Config::default());
    finder.refresh();

    finder.set_snapshot(world_with(vec![pane(1, true)]));
    finder.refresh();

    assert_eq!(
        finder.selection().selected_key(),
        Some(&NodeKey::Pane {
            session: "work".into(),
            tab_id: 42,
            pane_id: 1,
        })
    );
    assert!(finder.rows_ref().rows.iter().any(|row| {
        row.key
            == NodeKey::Pane {
                session: "work".into(),
                tab_id: 42,
                pane_id: 1,
            }
    }));
}

#[test]
fn foreground_flag_distinguishes_nested_shells_from_idle_shells() {
    let mut finder = Finder::new(Config::default());
    finder.set_snapshot(world_with(vec![pane(1, true)]));

    assert!(finder.set_command_with_foreground("work", 1, vec!["zsh".into()], true));
    assert!(finder.has_running_visible_pane());

    assert!(finder.set_command_with_foreground("work", 1, vec!["zsh".into()], false));
    assert!(!finder.has_running_visible_pane());
}

#[test]
fn non_template_pane_value_updates_do_not_rebuild_rows() {
    let config = Config::from_map(&BTreeMap::from([("row_format".into(), "{title}".into())]));
    let mut finder = Finder::new(config);
    finder.set_snapshot(world_with(vec![pane(1, true)]));
    finder.refresh();
    let rebuilds = finder.rebuild_count();

    assert!(finder.set_command_with_foreground(
        "work",
        1,
        vec!["cargo".into(), "test".into()],
        true,
    ));
    assert!(finder.set_cwd("work", 1, PathBuf::from("/tmp/work")));
    finder.refresh();

    assert_eq!(finder.rebuild_count(), rebuilds);
}

#[test]
fn spinner_and_vertical_selection_reuse_rows() {
    let mut finder = Finder::new(Config::default());
    finder.set_snapshot(world_with(vec![pane(1, true), pane(2, false)]));
    finder.refresh();
    let rebuilds = finder.rebuild_count();

    finder.advance_spinner();
    finder.selection_down();
    finder.selection_up();
    finder.refresh();

    assert_eq!(finder.rebuild_count(), rebuilds);
}
