
use std::io;
use std::time::{Duration, Instant};

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Widget};
use ratatui::Terminal;
use tachyonfx::{fx, Duration as FxDuration, Effect, EffectTimer, Interpolation};

use crate::theme::{ACCENT, FG, MUTED};

const FRAME_MS: u64 = 16;
const MAX_SPLASH_MS: u64 = 1500;
const FADE_IN_MS: u32 = 600;
const MIN_COLS_SMALL: u16 = 32;
const MIN_COLS_MEDIUM: u16 = 48;
const MIN_COLS_LARGE: u16 = 90;
const MIN_ROWS: u16 = 12;

const WORDMARK_SMALL: &str = "\
█▀█ █   █▀▀ ▀▄▀ █ █ █▀▀\n\
█▀▀ █▄▄ ██▄ █ █ █▄█ ▄██";

const WORDMARK_MEDIUM: &str = "\
 ____  _     _______  ___   _ ____\n\
|  _ \\| |   | ____\\ \\/ / | | / ___|\n\
| |_) | |   |  _|  \\  /| | | \\___ \\\n\
|  __/| |___| |___ /  \\| |_| |___) |\n\
|_|   |_____|_____/_/\\_\\\\___/|____/";

const WORDMARK_LARGE: &str = "\
██████╗ ██╗     ███████╗██╗  ██╗██╗   ██╗███████╗\n\
██╔══██╗██║     ██╔════╝╚██╗██╔╝██║   ██║██╔════╝\n\
██████╔╝██║     █████╗   ╚███╔╝ ██║   ██║███████╗\n\
██╔═══╝ ██║     ██╔══╝   ██╔██╗ ██║   ██║╚════██║\n\
██║     ███████╗███████╗██╔╝ ██╗╚██████╔╝███████║\n\
╚═╝     ╚══════╝╚══════╝╚═╝  ╚═╝ ╚═════╝ ╚══════╝";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WordmarkSize {
    Small,
    Medium,
    Large,
}

impl WordmarkSize {
    fn pick(width: u16) -> Self {
        if width >= MIN_COLS_LARGE {
            WordmarkSize::Large
        } else if width >= MIN_COLS_MEDIUM {
            WordmarkSize::Medium
        } else {
            WordmarkSize::Small
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            WordmarkSize::Small => WORDMARK_SMALL,
            WordmarkSize::Medium => WORDMARK_MEDIUM,
            WordmarkSize::Large => WORDMARK_LARGE,
        }
    }

    fn rendered(self) -> String {
        self.as_str()
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn line_count(self) -> usize {
        self.rendered().lines().count()
    }
}

pub fn should_show_splash(value: Option<bool>) -> bool {
    value.unwrap_or(true)
}

pub fn read_show_splash() -> bool {
    let path = match crate::prefs::Prefs::config_path() {
        Some(p) => p,
        None => return true,
    };
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return true,
    };
    let json: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return true,
    };
    should_show_splash(json.get("show_splash").and_then(|v| v.as_bool()))
}

fn draw_wordmark(area: Rect, buf: &mut Buffer) {
    let size = WordmarkSize::pick(area.width);
    let wordmark = size.rendered();
    let line_count = size.line_count() as u16;
    let chunks = Layout::vertical([
        Constraint::Length(line_count),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .flex(Flex::Center)
    .split(area);
    let title = chunks[0];
    let tagline = chunks[2];
    let lines: Vec<Line> = wordmark
        .lines()
        .map(|l| Line::from(l.to_string()).style(Style::default().fg(ACCENT)))
        .collect();
    Paragraph::new(lines)
        .alignment(Alignment::Center)
        .render(title, buf);
    Paragraph::new(
        Line::from("TERMINAL MULTIPLEXER · v0.1.0").style(Style::default().fg(MUTED)),
    )
    .alignment(Alignment::Center)
    .render(tagline, buf);
}

pub fn run_splash<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
) -> io::Result<()> {
    let area = match terminal.size() {
        Ok(a) => a,
        Err(_) => return Ok(()),
    };
    if area.width < MIN_COLS_SMALL || area.height < MIN_ROWS {
        return Ok(());
    }

    let mut effect: Effect = fx::fade_from_fg(
        FG,
        EffectTimer::from_ms(FADE_IN_MS, Interpolation::QuadInOut),
    );

    let start = Instant::now();
    let mut last = start;
    loop {
        if start.elapsed() >= Duration::from_millis(MAX_SPLASH_MS) {
            break;
        }
        if crossterm::event::poll(Duration::from_millis(FRAME_MS)).unwrap_or(false) {
            let _ = crossterm::event::read();
            break;
        }
        let now = Instant::now();
        let delta = now.duration_since(last);
        last = now;
        let fx_delta = FxDuration::from_millis(delta.as_millis() as u32);

        let draw_res = terminal.draw(|frame| {
            let area = frame.area();
            let buf = frame.buffer_mut();
            draw_wordmark(area, buf);
            effect.process(fx_delta, buf, area);
        });
        if draw_res.is_err() {
            return Ok(());
        }

        if effect.done() {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn wordmarks_line_counts() {
        assert_eq!(WordmarkSize::Small.line_count(), 2);
        assert_eq!(WordmarkSize::Medium.line_count(), 5);
        assert_eq!(WordmarkSize::Large.line_count(), 6);
    }

    #[test]
    fn rendered_wordmarks_have_no_trailing_whitespace() {
        for s in [WordmarkSize::Small, WordmarkSize::Medium, WordmarkSize::Large] {
            for (i, line) in s.rendered().lines().enumerate() {
                assert_eq!(
                    line,
                    line.trim_end(),
                    "{:?} rendered line {i} has trailing space: {line:?}",
                    s
                );
            }
        }
    }

    #[test]
    fn wordmarks_rendered_width_increases_with_size() {
        let sw = WordmarkSize::Small
            .rendered()
            .lines()
            .map(|l| l.chars().count())
            .max();
        let mw = WordmarkSize::Medium
            .rendered()
            .lines()
            .map(|l| l.chars().count())
            .max();
        let lw = WordmarkSize::Large
            .rendered()
            .lines()
            .map(|l| l.chars().count())
            .max();
        assert!(sw.is_some() && mw.is_some() && lw.is_some());
        assert!(sw.unwrap() < mw.unwrap());
        assert!(mw.unwrap() < lw.unwrap());
    }

    #[test]
    fn pick_chooses_size_by_width() {
        assert_eq!(WordmarkSize::pick(31), WordmarkSize::Small);
        assert_eq!(WordmarkSize::pick(32), WordmarkSize::Small);
        assert_eq!(WordmarkSize::pick(47), WordmarkSize::Small);
        assert_eq!(WordmarkSize::pick(48), WordmarkSize::Medium);
        assert_eq!(WordmarkSize::pick(89), WordmarkSize::Medium);
        assert_eq!(WordmarkSize::pick(90), WordmarkSize::Large);
        assert_eq!(WordmarkSize::pick(200), WordmarkSize::Large);
    }

    #[test]
    fn should_show_splash_defaults_true() {
        assert!(should_show_splash(None));
    }

    #[test]
    fn should_show_splash_respects_explicit_false() {
        assert!(!should_show_splash(Some(false)));
        assert!(should_show_splash(Some(true)));
    }

    #[test]
    fn draw_wordmark_writes_content_into_buffer() {
        let backend = TestBackend::new(120, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                let buf = frame.buffer_mut();
                draw_wordmark(area, buf);
            })
            .unwrap();
        let dump = format!("{:?}", terminal.backend().buffer());
        let has_letter_glyph = dump
            .chars()
            .any(|c| matches!(c, 'P' | 'L' | 'E' | 'X' | 'U' | 'S' | '_' | '|' | '/' | '█' | '▀' | '▄' | '╔' | '═' | '╗' | '╚' | '╝'));
        assert!(
            has_letter_glyph,
            "buffer should contain PLEXUS glyphs; got empty/blank"
        );
    }

    #[test]
    fn draw_wordmark_responsive_at_60_cols() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                let buf = frame.buffer_mut();
                draw_wordmark(area, buf);
            })
            .unwrap();
        let dump = format!("{:?}", terminal.backend().buffer());
        let has_letter = dump
            .chars()
            .any(|c| matches!(c, 'P' | 'L' | 'E' | 'X' | 'U' | 'S' | '█' | '▀' | '▄' | '╔' | '═' | '╗' | '╚' | '╝' | '_'));
        assert!(has_letter, "60-col terminal should still render a wordmark");
    }

    #[test]
    fn draw_wordmark_is_centered_vertically() {
        let height = 30;
        let backend = TestBackend::new(120, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                let buf = frame.buffer_mut();
                draw_wordmark(area, buf);
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        let first_row_has_content = (0..120).any(|x| buf[(x, 0)].symbol() != " ");
        let center_row_has_content = (0..120).any(|x| buf[(x, height / 2)].symbol() != " ");
        assert!(
            !first_row_has_content,
            "Top row should be blank padding when vertically centered"
        );
        assert!(
            center_row_has_content,
            "Center row should contain content when vertically centered"
        );
    }

    #[test]
    #[ignore = "visual dump: run with `cargo test print_wordmarks -- --ignored --nocapture`"]
    fn print_wordmarks() {
        for size in [WordmarkSize::Small, WordmarkSize::Medium, WordmarkSize::Large] {
            println!("\n--- {:?} ---", size);
            for line in size.rendered().lines() {
                println!("|{}|  (len={})", line, line.len());
            }
        }
    }
}

