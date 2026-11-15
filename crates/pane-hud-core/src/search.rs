#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct MatchScore {
    pub value: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchResult {
    pub score: MatchScore,
    pub positions: Vec<usize>,
}

pub fn normalize_query(query: &str) -> String {
    query.trim().to_lowercase()
}

pub fn score_substring(query: &str, text: &str) -> Option<MatchScore> {
    match_substring(query, text).map(|result| result.score)
}

pub fn match_substring(query: &str, text: &str) -> Option<MatchResult> {
    match_substring_lowercase(&query.to_lowercase(), &text.to_lowercase())
}

pub(crate) fn match_substring_lowercase(query: &str, text: &str) -> Option<MatchResult> {
    if query.is_empty() {
        return Some(MatchResult {
            score: MatchScore::default(),
            positions: Vec::new(),
        });
    }
    let byte_start = text.find(query)?;
    let char_start = text[..byte_start].chars().count() as u32;
    let matched = query.chars().count() as u32;
    let query_len = query.chars().count() as u32;
    let value = 1_200 + matched.saturating_mul(60) - char_start.saturating_mul(4);
    let positions = (char_start as usize..char_start as usize + query.chars().count()).collect();
    Some(MatchResult {
        score: MatchScore {
            value: value.saturating_add(300_u32.saturating_sub(query_len.saturating_mul(2))),
        },
        positions,
    })
}

pub fn score_subsequence(query: &str, text: &str) -> Option<MatchScore> {
    match_subsequence(query, text).map(|result| result.score)
}

pub fn match_subsequence(query: &str, text: &str) -> Option<MatchResult> {
    match_subsequence_lowercase(&query.to_lowercase(), &text.to_lowercase())
}

pub(crate) fn match_subsequence_lowercase(query: &str, text: &str) -> Option<MatchResult> {
    if query.is_empty() {
        return Some(MatchResult {
            score: MatchScore::default(),
            positions: Vec::new(),
        });
    }

    let query_chars: Vec<char> = query.chars().collect();
    let text_chars: Vec<char> = text.chars().collect();
    let mut query_index = 0;
    let mut matched_positions = Vec::with_capacity(query_chars.len());
    let mut previous_is_boundary = true;
    let mut boundaries = Vec::with_capacity(text_chars.len());

    for (text_index, character) in text_chars.iter().enumerate() {
        boundaries.push(previous_is_boundary);
        if query_index < query_chars.len() && *character == query_chars[query_index] {
            matched_positions.push(text_index);
            query_index += 1;
        }
        previous_is_boundary = !character.is_alphanumeric();
    }
    if query_index != query_chars.len() {
        return None;
    }

    let first = *matched_positions.first().unwrap_or(&0);
    let last = *matched_positions.last().unwrap_or(&0);
    let consecutive = matched_positions
        .windows(2)
        .filter(|window| window[1] == window[0] + 1)
        .count() as u32;
    let boundary_matches = matched_positions
        .iter()
        .filter(|position| boundaries.get(**position).copied().unwrap_or(false))
        .count() as u32;
    let span = last.saturating_sub(first) + 1;
    let gaps = span.saturating_sub(matched_positions.len()) as u32;

    let base = 1_000_u32
        .saturating_add((matched_positions.len() as u32).saturating_mul(45))
        .saturating_add(consecutive.saturating_mul(70))
        .saturating_add(boundary_matches.saturating_mul(25))
        .saturating_add(80_u32.saturating_sub((text_chars.len() as u32).min(80)));
    let penalty = (first as u32)
        .saturating_mul(4)
        .saturating_add(gaps.saturating_mul(10));

    Some(MatchResult {
        score: MatchScore {
            value: base.saturating_sub(penalty),
        },
        positions: matched_positions,
    })
}
