
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

pub fn ansi_spans(text: &str, base: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut style = base;
    let mut buf = String::new();
    let mut chars = text.chars().peekable();
    let flush = |buf: &mut String, style: Style, spans: &mut Vec<Span<'static>>| {
        if !buf.is_empty() {
            spans.push(Span::styled(std::mem::take(buf), style));
        }
    };
    while let Some(c) = chars.next() {
        if c == '\x1b' {

            if chars.next() == Some('[') {
                let mut params = String::new();
                for p in chars.by_ref() {
                    if p == 'm' {
                        break;
                    }
                    params.push(p);
                }
                flush(&mut buf, style, &mut spans);
                if params == "0" || params == "22" || params == "39" {
                    style = base;
                } else if params == "1" {
                    style = style.add_modifier(Modifier::BOLD);
                } else if let Some(rest) = params.strip_prefix("38;2;") {
                    let mut it = rest.split(';');
                    let r: u8 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                    let g: u8 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                    let b: u8 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                    style = style.fg(ratatui::style::Color::Rgb(r, g, b));
                }
                continue;
            }
            buf.push('\x1b');
        } else {
            buf.push(c);
        }
    }
    flush(&mut buf, style, &mut spans);
    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn parses_truecolor_bold_and_reset() {
        let base = Style::default().fg(Color::Rgb(200, 200, 200));
        let spans = ansi_spans("\x1b[1mBold\x1b[0m \x1b[38;2;10;20;30mRGB\x1b[39m tail", base);
        let texts: Vec<String> = spans.iter().map(|s| s.content.to_string()).collect();
        assert_eq!(texts, vec!["Bold", " ", "RGB", " tail"]);
        assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(
            spans[2].style.fg,
            Some(Color::Rgb(10, 20, 30)),
            "truecolor applied"
        );

        assert_eq!(spans[3].style.fg, base.fg);

        assert!(!spans[3].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn skips_unknown_codes_and_lone_escapes() {
        let base = Style::default();
        let spans = ansi_spans("\x1b[99mhi\x1b", base);
        let texts: Vec<String> = spans.iter().map(|s| s.content.to_string()).collect();
        assert_eq!(texts, vec!["hi\u{1b}"]);
    }
}

