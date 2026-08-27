
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use crate::theme::Palette;
use crate::ui::pane::MuxPane;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabGeom {

    pub start_x: u16,

    pub body_x: u16,

    pub body_len: u16,

    pub width: u16,
}

fn truncate_columns(s: &str, max_cols: usize) -> String {
    use unicode_width::UnicodeWidthChar;
    let mut cols = 0usize;
    let mut out = String::new();
    for ch in s.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(1).max(1);
        if cols + w > max_cols.saturating_sub(1) {
            out.push('…');
            return out;
        }
        out.push(ch);
        cols += w;
    }
    out
}

fn str_columns(s: &str) -> u16 {
    use unicode_width::UnicodeWidthStr;
    s.width() as u16
}

pub fn tab_geometries(area: Rect, titles: &[String], closable: bool) -> Vec<TabGeom> {
    let max_x = area.right().saturating_sub(2);
    let mut x = area.left();
    let mut out = Vec::new();
    for (idx, title) in titles.iter().enumerate() {
        let title_max = if titles.len() > 6 {
            6
        } else if titles.len() > 3 {
            10
        } else {
            18
        };
        let formatted = if title.is_empty() || title == "commandcode" {
            format!("Terminal {}", idx + 1)
        } else if str_columns(title) > title_max as u16 {
            truncate_columns(title, title_max)
        } else {
            title.clone()
        };
        let close_glyph = if closable { " ✕" } else { "" };

        let body = format!(" ◈ {}{} ", formatted, close_glyph);

        let body_len = str_columns(&body);

        let width = body_len + 2;
        if x + width > max_x {
            break;
        }
        out.push(TabGeom {
            start_x: x,
            body_x: x + 1,
            body_len,
            width,
        });
        x += width;
    }
    out
}

pub fn render_tab_bar(
    frame: &mut ratatui::Frame,
    area: Rect,
    panes: &[Arc<Mutex<MuxPane>>],
    active: usize,
) {
    let p = Palette::dark();
    let bg = crate::theme::effective_bg();

    {
        let buf = frame.buffer_mut();
        let width = area.width as usize;
        let spaces = " ".repeat(width);
        for y in area.top()..area.bottom() {
            buf.set_string(area.left(), y, &spaces, Style::default().bg(bg));
        }
    }

    let titles: Vec<String> = panes
        .iter()
        .map(|pane| pane.lock().unwrap_or_else(|e| e.into_inner()).state.title.clone())
        .collect();
    let closable = panes.len() > 1;
    let geoms = tab_geometries(area, &titles, closable);

    let active_bg = p.accent;
    let inactive_bg = p.surface0;
    let btn_bg = p.surface1;

    for (idx, geom) in geoms.iter().enumerate() {
        let raw_title = &titles[idx];

        let title_max = if panes.len() > 6 {
            6
        } else if panes.len() > 3 {
            10
        } else {
            18
        };
        let formatted_title = if raw_title.is_empty() || raw_title == "commandcode" {
            format!("Terminal {}", idx + 1)
        } else if str_columns(raw_title) > title_max as u16 {
            truncate_columns(raw_title, title_max)
        } else {
            raw_title.clone()
        };

        let pane_guard = panes[idx].lock().unwrap_or_else(|e| e.into_inner());
        let agent_state = pane_guard.state.agent_state;
        drop(pane_guard);
        let spinner = {
            let ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            ["|", "/", "-", "\\"][(ms / 120) as usize % 4]
        };
        let idle_blue = p.blue;
        let (icon, icon_color) = match agent_state {
            crate::agent_state::AgentState::Working => (spinner, idle_blue),
            crate::agent_state::AgentState::Blocked => {

                let blink = (SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
                    / 600)
                    % 2
                    == 0;
                (if blink { "!" } else { " " }, p.red)
            }

            crate::agent_state::AgentState::Idle => ("◈", idle_blue),
        };
        let selected = idx == active;
        let close_glyph = if closable { " ✕" } else { "" };

        let tab_body = format!(" {} {}{} ", icon, formatted_title, close_glyph);
        let body_len = str_columns(&tab_body);
        let x = geom.start_x;

        let curr_bg = if selected { active_bg } else { inactive_bg };
        let next_bg = if idx + 1 < geoms.len() {
            if (idx + 1) == active {
                active_bg
            } else {
                inactive_bg
            }
        } else {
            let plus_x = geom.start_x + geom.width;
            if plus_x + 4 <= max_x_of(area) {
                btn_bg
            } else {
                bg
            }
        };

        let inactive_text = p.overlay1;
        let inactive_icon = p.subtext0;

        let buf = frame.buffer_mut();
        let y = area.top();

        let mut col = 0u16;
        let total_cols = body_len;
        for ch in tab_body.chars() {
            let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1).max(1) as u16;
            let cx = x + col;
            let is_close = closable
                && col >= total_cols.saturating_sub(3)
                && col < total_cols.saturating_sub(1);

            let style = if is_close {
                Style::default()
                    .fg(if selected { p.red } else { inactive_icon })
                    .bg(curr_bg)
                    .add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    })
            } else if col <= 1 {

                let fg = if selected { p.panel_bg } else { icon_color };
                Style::default()
                    .fg(fg)
                    .bg(curr_bg)
                    .add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    })
            } else {
                Style::default()
                    .fg(if selected { p.text } else { inactive_text })
                    .bg(curr_bg)
                    .add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    })
            };

            buf[(cx, y)].set_symbol(&ch.to_string()).set_style(style);
            col += w;
        }

        let rx = x + body_len;
        buf[(rx, y)]
            .set_symbol("")
            .set_style(Style::default().fg(curr_bg).bg(next_bg));

        buf[(rx + 1, y)]
            .set_symbol(" ")
            .set_style(Style::default().bg(next_bg));
    }

    let plus_x = geoms
        .last()
        .map(|g| g.start_x + g.width)
        .unwrap_or(area.left());
    if plus_x + 4 <= max_x_of(area) {
        let buf = frame.buffer_mut();
        let y = area.top();
        buf[(plus_x, y)]
            .set_symbol(" ")
            .set_style(Style::default().bg(btn_bg));

        buf[(plus_x + 1, y)].set_symbol("+").set_style(
            Style::default()
                .fg(p.green)
                .bg(btn_bg)
                .add_modifier(Modifier::BOLD),
        );
        buf[(plus_x + 2, y)]
            .set_symbol(" ")
            .set_style(Style::default().bg(btn_bg));
        buf[(plus_x + 3, y)]
            .set_symbol("")
            .set_style(Style::default().fg(btn_bg).bg(bg));
    }
}

fn max_x_of(area: Rect) -> u16 {
    area.right().saturating_sub(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_columns_counts_wide_chars_as_two() {

        assert_eq!(truncate_columns("terminal one", 6), "termi…");
        assert_eq!(truncate_columns("hello", 5), "hell…");

        assert_eq!(truncate_columns("日本terminal", 6), "日本t…");

        assert_eq!(truncate_columns("日本", 3), "日…");

        assert_eq!(truncate_columns("ab", 4), "ab");
    }

    fn rect(w: u16) -> Rect {
        Rect::new(0, 0, w, 1)
    }

    #[test]
    fn tabs_are_contiguous_and_do_not_overlap() {
        let titles = vec![
            "commandcode".to_string(),
            "cargo build".to_string(),
            "git status".to_string(),
        ];
        let geoms = tab_geometries(rect(120), &titles, true);
        assert_eq!(geoms.len(), 3);
        for pair in geoms.windows(2) {
            assert_eq!(pair[1].start_x, pair[0].start_x + pair[0].width);
        }
    }

    #[test]
    fn single_tab_has_no_close_glyph_but_still_fits() {
        let titles = vec!["commandcode".to_string()];
        let geoms = tab_geometries(rect(80), &titles, false);
        assert_eq!(geoms.len(), 1);
        assert_eq!(geoms[0].body_len, 14);
    }

    #[test]
    fn click_hit_test_matches_geometry() {
        let titles = vec!["commandcode".to_string(), "second".to_string()];
        let geoms = tab_geometries(rect(100), &titles, true);
        let mid0 = geoms[0].body_x + geoms[0].body_len / 2;
        assert!(mid0 >= geoms[0].body_x && mid0 < geoms[0].body_x + geoms[0].body_len);
    }

    #[test]
    fn many_tabs_break_off_cleanly_when_out_of_width() {
        let titles: Vec<String> = (0..10).map(|i| format!("session {}", i)).collect();
        let geoms = tab_geometries(rect(60), &titles, true);
        assert!(!geoms.is_empty());
        assert!(geoms.len() < 10);
        for g in &geoms {
            assert!(g.start_x + g.width <= 60);
        }
    }
}

