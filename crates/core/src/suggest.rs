//! "Did you mean": the nearest name to a misspelt one, for error messages that list valid options.

/// The candidate closest to `word` (edit distance at most a third of its length, at least 2).
pub fn suggest(word: &str, candidates: &[&str]) -> Option<String> {
    fn distance(a: &str, b: &str) -> usize {
        let b: Vec<char> = b.chars().collect();
        let mut prev: Vec<usize> = (0..=b.len()).collect();
        for (i, ca) in a.chars().enumerate() {
            let mut cur = vec![i + 1];
            for (j, cb) in b.iter().enumerate() {
                cur.push((prev[j] + usize::from(ca != *cb)).min(prev[j + 1] + 1).min(cur[j] + 1));
            }
            prev = cur;
        }
        prev[b.len()]
    }
    let limit = (word.chars().count() / 3).max(2);
    candidates.iter().map(|c| (distance(word, c), *c)).filter(|(d, _)| *d <= limit).min_by_key(|(d, _)| *d).map(|(_, c)| c.to_string())
}

/// ` Did you mean "x"?` for the nearest candidate, or nothing.
pub fn hint(word: &str, candidates: &[&str]) -> String {
    suggest(word, candidates).map_or(String::new(), |s| format!(" Did you mean {s:?}?"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_near_misses_only() {
        assert_eq!(suggest("halftoen", &["halftone", "vhs"]).as_deref(), Some("halftone"));
        assert_eq!(suggest("zzzzzz", &["halftone", "vhs"]), None);
    }

    #[test]
    fn the_hint_is_a_sentence_or_empty() {
        assert_eq!(hint("halftoen", &["halftone"]), " Did you mean \"halftone\"?");
        assert_eq!(hint("zzzzzz", &["halftone"]), "");
    }
}
