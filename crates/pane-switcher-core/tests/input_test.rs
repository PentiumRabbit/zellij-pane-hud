use pane_switcher_core::input::{Selection, TextQuery};
use pane_switcher_core::model::{LivePane, NodeKey};
use pane_switcher_core::rows::{FieldMatches, ProgressState, Row, Rows};

fn key(session: &str, id: u32) -> NodeKey {
    NodeKey::Pane {
        session: session.into(),
        tab_id: 1,
        pane_id: id,
    }
}

fn row(key: NodeKey) -> Row {
    let pane = LivePane {
        key: key.clone(),
        id: key.pane_id().unwrap_or_default(),
        title: "row".into(),
        is_focused: false,
        is_floating: false,
        is_fullscreen: false,
        is_suppressed: false,
        exited: false,
        initial_command: None,
    };
    Row {
        key,
        session: "work".into(),
        tab_name: "code".into(),
        tab_position: 0,
        title: "row".into(),
        dir: ".".into(),
        full_dir: ".".into(),
        command: "zsh".into(),
        full_command: "zsh".into(),
        pane,
        is_current_session: true,
        progress: ProgressState::Idle,
        score: 0,
        matches: FieldMatches::default(),
    }
}

#[test]
fn text_query_edits_utf8_and_words_without_byte_slicing() {
    let mut query = TextQuery::default();
    query.insert_char('测');
    query.insert_char('试');
    query.insert_char(' ');
    query.insert_char('a');
    query.backspace();
    assert_eq!(query.as_str(), "测试 ");

    query.insert_str("foo bar  ");
    query.delete_word();
    assert_eq!(query.as_str(), "测试 foo ");

    query.clear();
    assert!(query.is_empty());
}

#[test]
fn selection_wraps_and_keeps_a_stable_key_after_refresh() {
    let rows = Rows {
        rows: vec![row(key("work", 1)), row(key("work", 2))],
    };
    let mut selection = Selection::default();
    selection.up(&rows);
    assert_eq!(selection.index(), 1);
    selection.down(&rows);
    assert_eq!(selection.index(), 0);

    selection.select_key(&rows, &key("work", 2));
    let refreshed = Rows {
        rows: vec![row(key("work", 3)), row(key("work", 2))],
    };
    selection.restore(&refreshed, None);
    assert_eq!(selection.index(), 1);
}
