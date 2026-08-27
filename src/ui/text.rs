
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub fn width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

pub fn truncate(s: &str, max: usize) -> String {
    if width(s) <= max {
        return s.to_string();
    }
    if max == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut used = 0usize;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        let cw = UnicodeWidthChar::width(c).unwrap_or(1);
        if used + cw + 1 > max {
            break;
        }
        out.push(c);
        used += cw;
        i += 1;
    }

    while out
        .chars()
        .last()
        .map(|c| UnicodeWidthChar::width(c) == Some(0))
        .unwrap_or(false)
    {
        out.pop();
    }
    if used == 0 && !chars.is_empty() {

        return "…".to_string();
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_counts_cjk_as_two() {
        assert_eq!(width("ab"), 2);
        assert_eq!(width("é"), 1);
        assert_eq!(width("e\u{301}"), 1);
        assert_eq!(width("日"), 2);
        assert_eq!(width("a日"), 3);
    }

    #[test]
    fn truncate_is_grapheme_safe() {

        let s = "eeee\u{301}eeee";
        let t = truncate(s, 6);
        assert!(!t.ends_with('\u{301}'), "no trailing combining mark: {t:?}");
        assert!(width(&t) <= 7, "truncated width within ellipsis budget");
        assert!(t.ends_with('…'));
    }

    #[test]
    fn truncate_cjk_and_pua() {
        assert_eq!(truncate("abcde", 3), "ab…");
        assert_eq!(truncate("日本語です", 5), "日本…");
        assert_eq!(width(&truncate("日本語です", 5)), 5);
        let pua = "\u{eaf8}";
        assert_eq!(width(pua), 1);
        assert_eq!(truncate("ok", 0), "");
    }

    #[test]
    fn truncate_no_op_when_fits() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("", 5), "");
    }
}

