use std::collections::HashMap;

use pane_hud_core::app::NavigationTarget;
use pane_hud_core::model::NodeKey;
use pane_hud_core::style::{ColorSpec, ThemePalette};
use pane_hud_plugin::adapter::{
    key_intent, navigation_action, pane_manifest_input, session_input, tab_inputs, theme_palette,
    InputIntent, NavigationAction,
};
use zellij_tile::prelude::{
    BareKey, KeyWithModifier, PaletteColor, PaneId, PaneInfo, PaneManifest, SessionInfo, Style,
    TabInfo,
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

    let mut world = pane_hud_core::model::World::default();
    world.apply_snapshot(
        vec![pane_hud_core::model::SessionInput {
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

#[test]
fn zellij_style_colors_map_to_pane_hud_semantic_palette() {
    let mut style = Style::default();
    let colors = &mut style.colors;
    colors.text_unselected.base = PaletteColor::Rgb((192, 202, 245));
    colors.text_unselected.emphasis_0 = PaletteColor::Rgb((255, 158, 100));
    colors.text_unselected.emphasis_1 = PaletteColor::Rgb((125, 207, 255));
    colors.text_unselected.emphasis_2 = PaletteColor::Rgb((158, 206, 106));
    colors.text_unselected.emphasis_3 = PaletteColor::Rgb((187, 154, 247));
    colors.exit_code_error.base = PaletteColor::Rgb((247, 118, 142));
    colors.multiplayer_user_colors.player_2 = PaletteColor::Rgb((122, 162, 247));
    colors.multiplayer_user_colors.player_4 = PaletteColor::Rgb((224, 175, 104));
    colors.multiplayer_user_colors.player_8 = PaletteColor::Rgb((160, 160, 180));
    colors.text_selected.base = PaletteColor::Rgb((200, 210, 250));
    colors.text_selected.background = PaletteColor::Rgb((26, 27, 38));
    colors.frame_selected.base = PaletteColor::Rgb((158, 206, 106));

    let palette = theme_palette(&style);

    assert_eq!(
        palette,
        ThemePalette {
            default: ColorSpec::Rgb((192, 202, 245)),
            gray: ColorSpec::Rgb((192, 202, 245)),
            green: ColorSpec::Rgb((158, 206, 106)),
            cyan: ColorSpec::Rgb((125, 207, 255)),
            magenta: ColorSpec::Rgb((187, 154, 247)),
            orange: ColorSpec::Rgb((255, 158, 100)),
            blue: ColorSpec::Rgb((122, 162, 247)),
            red: ColorSpec::Rgb((247, 118, 142)),
            yellow: ColorSpec::Rgb((224, 175, 104)),
            dark_gray: ColorSpec::Rgb((192, 202, 245)),
            border: ColorSpec::Rgb((158, 206, 106)),
            selected_foreground: ColorSpec::Rgb((200, 210, 250)),
            selected_background: ColorSpec::Rgb((26, 27, 38)),
        }
    );
}

#[test]
fn missing_zellij_silver_does_not_make_idle_icons_black() {
    let mut style = Style::default();
    style.colors.text_unselected.base = PaletteColor::Rgb((192, 202, 245));
    style.colors.multiplayer_user_colors.player_8 = PaletteColor::Rgb((0, 0, 0));

    let palette = theme_palette(&style);

    assert_eq!(palette.gray, ColorSpec::Rgb((192, 202, 245)));
    assert_eq!(palette.dark_gray, ColorSpec::Rgb((192, 202, 245)));
}
