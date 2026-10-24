//! Motion maths: easing, keyframed properties and expressions. Pure functions of time, no GPU, no Skia.

pub mod ease;
pub mod expr;
pub mod noise;
pub mod prop;

/// The candidate closest to `word` (edit distance at most a third of its length, at least 2), for "did you mean".
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

#[cfg(test)]
mod suggest_tests {
    use super::suggest;

    #[test]
    fn finds_near_misses_only() {
        assert_eq!(suggest("halftoen", &["halftone", "vhs"]).as_deref(), Some("halftone"));
        assert_eq!(suggest("zzzzzz", &["halftone", "vhs"]), None);
    }
}
