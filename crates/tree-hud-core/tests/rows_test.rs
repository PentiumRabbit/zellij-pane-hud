use std::collections::BTreeMap;
use std::path::PathBuf;

use tree_hud_core::config::Config;
use tree_hud_core::model::{
    NodeKey, PaneInput, PaneManifestInput, PaneValueCache, SessionInput, TabInput, World,
};
use tree_hud_core::rows::{ProgressState, Rows};

fn pane(id: u32, title: &str, focused: bool, initial_command: &str) -> PaneInput {
    PaneInput {
        id,
        title: title.into(),
        is_focused: focused,
        is_floating: false,
        is_fullscreen: false,
        is_suppressed: false,
        exited: false,
        is_selectable: true,
        is_plugin: false,
        initial_command: Some(initial_command.into()),
    }
}

fn fixture() -> World {
    let mut world = World::default();
    world.apply_snapshot(
        vec![
            SessionInput {
                name: "work".into(),
                is_current: true,
                connected_clients: 1,
                tabs: vec![
                    TabInput {
                        position: 0,
                        tab_id: 11,
                        name: "code".into(),
                        active: true,
                    },
                    TabInput {
                        position: 1,
                        tab_id: 12,
                        name: "tests".into(),
                        active: false,
                    },
                ],
                panes: PaneManifestInput {
                    panes: BTreeMap::from([
                        (
                            0,
                            vec![
                                pane(1, "nvim src/main.rs", true, "nvim src/main.rs"),
                                pane(2, "zsh", false, "zsh"),
                            ],
                        ),
                        (1, vec![pane(3, "integration tests", false, "cargo test")]),
                    ]),
                },
            },
            SessionInput {
                name: "archive".into(),
                is_current: false,
                connected_clients: 1,
                tabs: vec![TabInput {
                    position: 0,
                    tab_id: 21,
                    name: "notes".into(),
                    active: true,
                }],
                panes: PaneManifestInput {
                    panes: BTreeMap::from([(0, vec![pane(4, "journal", false, "hx")])]),
                },
            },
        ],
        Vec::new(),
    );
    world
}

fn config_with_format(format: &str) -> Config {
    Config::from_map(&BTreeMap::from([("row_format".into(), format.into())]))
}

#[test]
fn emits_each_pane_once_without_session_or_tab_rows() {
    let rows = Rows::build(
        &fixture(),
        &Config::default(),
        "",
        &PaneValueCache::default(),
    );

    assert_eq!(rows.rows.len(), 4);
    assert_eq!(
        rows.rows.iter().map(|row| row.pane.id).collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
    assert!(rows.rows.iter().all(|row| row.key.pane_id().is_some()));
}

#[test]
fn relative_dir_handles_equal_descendant_and_sibling_prefix_paths() {
    let mut world = World::default();
    world.apply_snapshot(
        vec![SessionInput {
            name: "work".into(),
            is_current: true,
            connected_clients: 1,
            tabs: vec![TabInput {
                position: 0,
                tab_id: 11,
                name: "code".into(),
                active: true,
            }],
            panes: PaneManifestInput {
                panes: BTreeMap::from([(
                    0,
                    vec![
                        pane(1, "base", true, "zsh"),
                        pane(2, "child", false, "zsh"),
                        pane(3, "sibling-prefix", false, "zsh"),
                        pane(4, "missing", false, "zsh"),
                    ],
                )]),
            },
        }],
        Vec::new(),
    );
    let mut values = PaneValueCache::default();
    values.set_cwd("work", 1, PathBuf::from("/work/app"));
    values.set_cwd("work", 2, PathBuf::from("/work/app/src"));
    values.set_cwd("work", 3, PathBuf::from("/work/application"));

    let rows = Rows::build(&world, &Config::default(), "", &values);
    let dirs = rows
        .rows
        .iter()
        .map(|row| row.dir.as_str())
        .collect::<Vec<_>>();

    assert_eq!(dirs, vec![".", "src", "/work/application", "?"]);
}

#[test]
fn template_fields_control_the_search_corpus() {
    let mut values = PaneValueCache::default();
    values.set_command("work", 2, vec!["cargo".into(), "test".into()]);
    let title_only = config_with_format("{title} {dir}");
    let with_command = config_with_format("{title} {dir} {command}");

    assert!(Rows::build(&fixture(), &title_only, "cargo", &values)
        .rows
        .is_empty());
    assert_eq!(
        Rows::build(&fixture(), &with_command, "cargo", &values)
            .rows
            .iter()
            .map(|row| row.pane.id)
            .collect::<Vec<_>>(),
        vec![2, 3]
    );
}

#[test]
fn search_records_utf8_positions_for_each_visible_field() {
    let config = config_with_format("{title} {dir}");
    let mut values = PaneValueCache::default();
    values.set_cwd("work", 1, PathBuf::from("/work/app"));
    values.set_cwd("work", 2, PathBuf::from("/work/app/src/ui"));

    let rows = Rows::build(&fixture(), &config, "ui", &values);
    assert_eq!(rows.rows.len(), 1);
    assert_eq!(rows.rows[0].pane.id, 2);
    assert_eq!(rows.rows[0].matches.dir, vec![4, 5]);

    let unicode = config_with_format("{title}");
    let mut unicode_world = World::default();
    unicode_world.apply_snapshot(
        vec![SessionInput {
            name: "work".into(),
            is_current: true,
            connected_clients: 1,
            tabs: vec![TabInput {
                position: 0,
                tab_id: 11,
                name: "code".into(),
                active: true,
            }],
            panes: PaneManifestInput {
                panes: BTreeMap::from([(0, vec![pane(1, "项目树HUD", true, "zsh")])]),
            },
        }],
        Vec::new(),
    );
    let rows = Rows::build(&unicode_world, &unicode, "树h", &PaneValueCache::default());
    assert_eq!(rows.rows[0].matches.title, vec![2, 3]);
}

#[test]
fn rows_fill_display_values_and_progress_state() {
    let mut values = PaneValueCache::default();
    values.set_command_with_foreground("work", 1, vec!["zsh".into()], true);
    values.set_cwd("work", 1, PathBuf::from("/work/app"));
    let rows = Rows::build(&fixture(), &Config::default(), "", &values);
    let first = &rows.rows[0];

    assert_eq!(first.key.session_name(), "work");
    assert_eq!(first.session, "work");
    assert_eq!(first.tab_name, "code");
    assert_eq!(first.tab_position, 0);
    assert_eq!(first.title, "nvim src/main.rs");
    assert_eq!(first.dir, ".");
    assert_eq!(first.command, "zsh");
    assert_eq!(first.progress, ProgressState::Running);
    assert!(first.is_current_session);
}

#[test]
fn results_sort_by_best_field_score_then_stable_traversal_order() {
    let config = config_with_format("{title}");
    let rows = Rows::build(&fixture(), &config, "test", &PaneValueCache::default());

    assert_eq!(
        rows.rows.iter().map(|row| row.pane.id).collect::<Vec<_>>(),
        vec![3]
    );
    assert!(rows.rows[0].score > 0);
}

#[test]
fn pane_rows_use_stable_navigation_keys() {
    let rows = Rows::build(
        &fixture(),
        &Config::default(),
        "",
        &PaneValueCache::default(),
    );

    assert_eq!(
        rows.rows[1].key,
        NodeKey::Pane {
            session: "work".into(),
            tab_id: 11,
            pane_id: 2,
        }
    );
}

#[test]
fn pane_id_is_searchable_when_the_template_contains_it() {
    let config = config_with_format("{pane_id} {title}");
    let rows = Rows::build(&fixture(), &config, "3", &PaneValueCache::default());

    assert_eq!(rows.rows.len(), 1);
    assert_eq!(rows.rows[0].pane.id, 3);
    assert_eq!(rows.rows[0].matches.pane_id, vec![0]);
}

#[test]
fn fuzzy_matches_can_span_adjacent_visible_fields() {
    let mut world = World::default();
    world.apply_snapshot(
        vec![SessionInput {
            name: "work".into(),
            is_current: true,
            connected_clients: 1,
            tabs: vec![TabInput {
                position: 0,
                tab_id: 11,
                name: "code".into(),
                active: true,
            }],
            panes: PaneManifestInput {
                panes: BTreeMap::from([(0, vec![pane(1, "worker 1", true, "task-0")])]),
            },
        }],
        Vec::new(),
    );
    let config = config_with_format("{title}  {command}");

    let rows = Rows::build(&world, &config, "10", &PaneValueCache::default());

    assert_eq!(rows.rows.len(), 1);
    assert_eq!(rows.rows[0].matches.title, vec![7]);
    assert_eq!(rows.rows[0].matches.command, vec![5]);
}
