use tree_hud_core::config::IconsMode;
use tree_hud_core::icons::{classify_pane, icon_for, progress_state, PaneVisualState};
use tree_hud_core::model::{LivePane, NodeKey};
use tree_hud_core::rows::ProgressState;

fn pane() -> LivePane {
    LivePane {
        key: NodeKey::Pane {
            session: "work".into(),
            tab_id: 1,
            pane_id: 7,
        },
        id: 7,
        title: "terminal".into(),
        is_focused: false,
        is_floating: false,
        is_fullscreen: false,
        is_suppressed: false,
        exited: false,
        initial_command: None,
    }
}

#[test]
fn exited_and_focused_states_take_precedence() {
    let mut exited = pane();
    exited.exited = true;
    exited.is_focused = true;
    assert_eq!(
        classify_pane(&exited, Some(&["codex".into()]), None, true),
        PaneVisualState::Exited
    );

    let mut focused = pane();
    focused.is_focused = true;
    assert_eq!(
        classify_pane(&focused, Some(&["nvim".into()]), None, true),
        PaneVisualState::Focused
    );
}

#[test]
fn command_categories_support_absolute_executable_paths() {
    assert_eq!(
        classify_pane(&pane(), Some(&["/usr/local/bin/codex".into()]), None, true,),
        PaneVisualState::Codex
    );
    assert_eq!(
        classify_pane(&pane(), Some(&["zsh".into()]), None, true),
        PaneVisualState::Shell
    );
    assert_eq!(
        classify_pane(
            &pane(),
            Some(&["nvim".into(), "src/main.rs".into()]),
            None,
            true,
        ),
        PaneVisualState::Editor
    );
}

#[test]
fn modifiers_and_unknown_are_used_when_command_state_is_missing() {
    let mut floating = pane();
    floating.is_floating = true;
    assert_eq!(
        classify_pane(&floating, None, None, false),
        PaneVisualState::Floating
    );
    assert_eq!(
        classify_pane(&pane(), None, None, false),
        PaneVisualState::Unknown
    );
}

#[test]
fn nerd_and_plain_modes_have_distinct_nonempty_icons() {
    let nerd = icon_for(PaneVisualState::Codex, IconsMode::Nerd);
    let plain = icon_for(PaneVisualState::Codex, IconsMode::Plain);
    assert_eq!(nerd, "❉");
    assert_eq!(plain, "CX");
    assert_ne!(nerd, plain);
}

#[test]
fn foreground_state_distinguishes_running_nested_shells_from_idle_shells() {
    let command = vec!["zsh".into()];

    assert_eq!(
        progress_state(&pane(), Some(&command), Some(true)),
        ProgressState::Running
    );
    assert_eq!(
        progress_state(&pane(), Some(&command), Some(false)),
        ProgressState::Idle
    );
}

#[test]
fn exited_and_unknown_progress_states_take_precedence() {
    let command = vec!["codex".into()];
    let mut exited = pane();
    exited.exited = true;

    assert_eq!(
        progress_state(&exited, Some(&command), Some(true)),
        ProgressState::Exited
    );
    assert_eq!(
        progress_state(&pane(), None, Some(true)),
        ProgressState::Unknown
    );
    assert_eq!(
        progress_state(&pane(), Some(&command), None),
        ProgressState::Running
    );
    assert_eq!(
        progress_state(&pane(), Some(&["zsh".into()]), None),
        ProgressState::Idle
    );
}
