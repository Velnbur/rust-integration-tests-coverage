//! Small text library used to check cross-crate coverage.

pub fn reverse(s: &str) -> String {
    s.chars().rev().collect()
}

pub fn is_palindrome(s: &str) -> bool {
    let cleaned: Vec<char> = s
        .chars()
        .filter(|c| c.is_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    cleaned.iter().eq(cleaned.iter().rev())
}

pub fn word_count(s: &str) -> usize {
    s.split_whitespace().count()
}

/// Intentionally NOT exercised by any test, so it must show up as uncovered.
pub fn shout(s: &str) -> String {
    format!("{}!", s.to_uppercase())
}
