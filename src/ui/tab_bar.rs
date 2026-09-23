
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

pub fn tab_geometries(
    area: Rect,
    titles: &[String],
    closable: bool,
    active: usize,
) -> Vec<TabGeom> {
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
        let is_selected = idx == active;
        let body = if is_selected {
            format!(" ◈ {}{} + ", formatted, close_glyph)
        } else {
            format!(" ◈ {}{} ", formatted, close_glyph)
        };

        let body_len = str_columns(&body);

        let tab_w = body_len + 4;
        let width = tab_w;
        if x + width > max_x {
            break;
        }
        out.push(TabGeom {
            start_x: x,
            body_x: x + 2,
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
    let f_area = frame.area();

    {
        let buf = frame.buffer_mut();
        let width = area.width as usize;
        let spaces = " ".repeat(width);
        for y in area.top()..=area.bottom().min(f_area.height.saturating_sub(1)) {
            buf.set_string(area.left(), y, &spaces, Style::default().bg(bg));
        }
    }

    let titles: Vec<String> = panes
        .iter()
        .map(|pane| pane.lock().unwrap_or_else(|e| e.into_inner()).state.title.clone())
        .collect();
    let closable = panes.len() > 1;
    let geoms = tab_geometries(area, &titles, closable, active);

    let active_idx = if active < geoms.len() { active } else { 0 };
    let border_color = p.accent;
    let border_style = Style::default().fg(border_color).bg(bg);
    let buf = frame.buffer_mut();
    let y = area.top();

    let mut set_cell = |x: u16, cy: u16, sym: &str, style: Style| {
        if x < f_area.width && cy < f_area.height {
            buf[(x, cy)].set_symbol(sym).set_style(style);
        }
    };

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

        let (icon, icon_color) = if let Some(pane_lock) = panes.get(idx) {
            let mut pane_guard = pane_lock.lock().unwrap_or_else(|e| e.into_inner());
            crate::ui::banner::ensure_boot_info(&mut pane_guard, area);
            let agent_state = pane_guard.state.agent_state;
            let spinner = {
                let ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis();
                ["|", "/", "-", "\\"][(ms / 120) as usize % 4]
            };
            let idle_blue = p.blue;
            match agent_state {
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
            }
        } else {
            ("◈", p.blue)
        };

        let selected = idx == active_idx;
        let close_glyph = if closable { " ✕" } else { "" };
        let tab_body = if selected {
            format!(" {} {}{} + ", icon, formatted_title, close_glyph)
        } else {
            format!(" {} {}{} ", icon, formatted_title, close_glyph)
        };
        let body_len = str_columns(&tab_body);
        let tab_w = body_len + 4;
        let right_x = geom.start_x + tab_w - 1;

        if selected {
            set_cell(geom.start_x, y, "╭", border_style);
            set_cell(geom.start_x + 1, y, "─", border_style);
            set_cell(right_x - 1, y, "─", border_style);
            set_cell(right_x, y, "╮", border_style);
        }

        let mut col = 0u16;
        let total_cols = body_len;
        for ch in tab_body.chars() {
            let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1).max(1) as u16;
            let cx = geom.body_x + col;
            let is_plus = selected && col == total_cols.saturating_sub(2);
            let is_close = closable
                && if selected {
                    col >= total_cols.saturating_sub(5) && col <= total_cols.saturating_sub(4)
                } else {
                    col >= total_cols.saturating_sub(3) && col < total_cols.saturating_sub(1)
                };

            let style = if is_plus {
                Style::default()
                    .fg(p.accent)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD)
            } else if is_close {
                Style::default()
                    .fg(if selected { p.red } else { p.subtext0 })
                    .bg(bg)
                    .add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    })
            } else if col <= 1 {
                Style::default()
                    .fg(if selected { icon_color } else { p.overlay0 })
                    .bg(bg)
                    .add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    })
            } else {
                Style::default()
                    .fg(if selected { p.text } else { p.subtext0 })
                    .bg(bg)
                    .add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    })
            };

            set_cell(cx, y, &ch.to_string(), style);
            col += w;
        }
    }

    if let Some(active_geom) = geoms.get(active_idx) {
        let row1 = area.bottom();
        let start_x = active_geom.start_x;
        let tab_w = active_geom.body_len + 4;
        let right_x = start_x + tab_w - 1;
        let max_x = area.right().saturating_sub(1);

        if start_x == area.left() {
            set_cell(area.left(), row1, "│", border_style);
        } else {
            set_cell(area.left(), row1, "╭", border_style);
            for cx in area.left() + 1..start_x {
                set_cell(cx, row1, "─", border_style);
            }
            set_cell(start_x, row1, "╯", border_style);
        }

        for cx in start_x + 1..right_x {
            set_cell(cx, row1, " ", Style::default().bg(bg));
        }

        set_cell(right_x, row1, "╰", border_style);

        for cx in right_x + 1..max_x {
            set_cell(cx, row1, "─", border_style);
        }
        set_cell(max_x, row1, "╮", border_style);
    }
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
        let geoms = tab_geometries(rect(120), &titles, true, 0);
        assert_eq!(geoms.len(), 3);
        for pair in geoms.windows(2) {
            assert_eq!(pair[1].start_x, pair[0].start_x + pair[0].width);
        }
    }

    #[test]
    fn single_tab_has_no_close_glyph_but_still_fits() {
        let titles = vec!["commandcode".to_string()];
        let geoms = tab_geometries(rect(80), &titles, false, 0);
        assert_eq!(geoms.len(), 1);
        assert_eq!(geoms[0].body_len, 16);
    }

    #[test]
    fn click_hit_test_matches_geometry() {
        let titles = vec!["commandcode".to_string(), "second".to_string()];
        let geoms = tab_geometries(rect(100), &titles, true, 0);
        let mid0 = geoms[0].body_x + geoms[0].body_len / 2;
        assert!(mid0 >= geoms[0].body_x && mid0 < geoms[0].body_x + geoms[0].body_len);
    }

    #[test]
    fn many_tabs_break_off_cleanly_when_out_of_width() {
        let titles: Vec<String> = (0..10).map(|i| format!("session {}", i)).collect();
        let geoms = tab_geometries(rect(60), &titles, true, 0);
        assert!(!geoms.is_empty());
        assert!(geoms.len() < 10);
        for g in &geoms {
            assert!(g.start_x + g.width <= 60);
        }
    }
}

