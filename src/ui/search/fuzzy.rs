
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuzzyMatch {
    pub score: i64,
    pub indices: Vec<usize>,
}

pub fn fuzzy_match(pattern: &str, text: &str) -> Option<FuzzyMatch> {
    if pattern.is_empty() {
        return Some(FuzzyMatch {
            score: 0,
            indices: Vec::new(),
        });
    }

    let pattern_chars: Vec<char> = pattern.chars().collect();
    let text_chars: Vec<char> = text.chars().collect();

    let pattern_lower: Vec<char> = pattern.to_lowercase().chars().collect();
    let text_lower: Vec<char> = text.to_lowercase().chars().collect();

    let mut p_idx = 0;
    let mut indices = Vec::with_capacity(pattern_chars.len());
    let mut score: i64 = 0;
    let mut consecutive: i64 = 0;
    let mut prev_idx: Option<usize> = None;

    for (t_idx, &t_ch) in text_lower.iter().enumerate() {
        if p_idx < pattern_lower.len() && t_ch == pattern_lower[p_idx] {
            indices.push(t_idx);

            if pattern_chars[p_idx] == text_chars[t_idx] {
                score += 5;
            }

            if t_idx == 0 || is_word_boundary(text_chars[t_idx - 1]) {
                score += 20;
            }

            if let Some(prev) = prev_idx {
                if t_idx == prev + 1 {
                    consecutive += 1;
                    score += 15 * consecutive;
                } else {
                    consecutive = 0;

                    score -= (t_idx - prev) as i64;
                }
            } else {

                score += (20 - t_idx.min(20)) as i64;
            }

            prev_idx = Some(t_idx);
            p_idx += 1;
        }
    }

    if p_idx == pattern_lower.len() {
        Some(FuzzyMatch { score, indices })
    } else {
        None
    }
}

fn is_word_boundary(c: char) -> bool {
    c == '/' || c == '\\' || c == '.' || c == '_' || c == '-' || c == ':' || c.is_whitespace()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_pattern_matches_everything() {
        assert_eq!(
            fuzzy_match("", "anything"),
            Some(FuzzyMatch {
                score: 0,
                indices: vec![]
            })
        );
    }

    #[test]
    fn exact_and_fuzzy_matches() {
        let m = fuzzy_match("pane", "src/ui/pane.rs");
        assert!(m.is_some());
        let m = m.unwrap();
        assert!(m.score > 0);
        assert_eq!(m.indices, vec![7, 8, 9, 10]);

        assert!(fuzzy_match("xyz", "src/ui/pane.rs").is_none());
    }

    #[test]
    fn word_boundary_scoring_priority() {
        let match1 = fuzzy_match("pr", "pane_render.rs").unwrap();
        let match2 = fuzzy_match("pr", "super_long_unrelated.rs").unwrap();
        assert!(match1.score > match2.score);
    }
}

