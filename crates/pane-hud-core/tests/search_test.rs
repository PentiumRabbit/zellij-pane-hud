use pane_hud_core::search::{
    match_subsequence, match_substring, normalize_query, score_subsequence, score_substring,
};

#[test]
fn subsequence_search_is_case_insensitive_for_unicode() {
    let query = normalize_query("ÜBER");
    assert_eq!(query, "über");
    assert!(score_subsequence(&query, "src/ÜberCool.rs").is_some());
}

#[test]
fn contiguous_matches_outrank_spread_matches() {
    let contiguous = score_subsequence("code", "codex").expect("contiguous match");
    let spread = score_subsequence("code", "c-old-d-e").expect("spread match");

    assert!(contiguous > spread);
}

#[test]
fn substring_search_rejects_missing_text() {
    assert!(score_substring("pane", "my pane title").is_some());
    assert!(score_substring("tab", "terminal").is_none());
}

#[test]
fn fuzzy_matches_return_utf8_character_positions() {
    let result = match_subsequence("树h", "项目树HUD").expect("match");

    assert_eq!(result.positions, vec![2, 3]);
}

#[test]
fn substring_matches_return_the_contiguous_character_range() {
    let result = match_substring("hud", "项目 HUD 面板").expect("match");

    assert_eq!(result.positions, vec![3, 4, 5]);
}
