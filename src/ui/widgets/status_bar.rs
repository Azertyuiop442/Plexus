
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::Frame;

#[derive(Debug, Clone)]
pub struct StatusSegment {
    pub text: String,
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
}

pub struct StatusBarWidget {
    pub left_segments: Vec<StatusSegment>,
    pub right_segments: Vec<StatusSegment>,
}

impl StatusBarWidget {
    pub fn new() -> Self {
        Self {
            left_segments: Vec::new(),
            right_segments: Vec::new(),
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, base_bg: Color) {
        if area.height < 1 || area.width < 2 {
            return;
        }
        let p = crate::theme::Palette::dark();
        let border_fg = p.blue;
        let line_style = Style::default().fg(border_fg).bg(base_bg);
        let bar_y = area.bottom().saturating_sub(1);
        let buf = frame.buffer_mut();
        let last_x = area.right().saturating_sub(1);

        if area.width > 0 {
            let spaces = " ".repeat(area.width as usize);
            buf.set_string(area.left(), bar_y, &spaces, Style::default().bg(base_bg));
        }

        buf[(area.left(), bar_y)].set_symbol("╰").set_style(line_style);
        buf[(last_x, bar_y)].set_symbol("╯").set_style(line_style);
        if area.width > 2 {
            buf[(area.left() + 1, bar_y)].set_symbol("─").set_style(line_style);
        }

        let mut x = area.left() + 3;
        let max_x = last_x.saturating_sub(2);

        let left_count = self.left_segments.len();
        for (idx, seg) in self.left_segments.iter().enumerate() {
            let mut style = Style::default().fg(seg.fg).bg(base_bg);
            if seg.bold {
                style = style.add_modifier(Modifier::BOLD);
            }

            let text = seg.text.trim();
            let text_len = text.chars().count() as u16;
            if x < max_x {
                buf.set_stringn(x, bar_y, text, (max_x - x) as usize, style);
                x += text_len;
            }

            if idx + 1 < left_count && x + 3 < max_x {
                let dot_style = Style::default().fg(border_fg).bg(base_bg);
                buf[(x, bar_y)].set_symbol(" ").set_style(dot_style);
                buf[(x + 1, bar_y)].set_symbol("·").set_style(dot_style);
                buf[(x + 2, bar_y)].set_symbol(" ").set_style(dot_style);
                x += 3;
            }
        }

        let right_count = self.right_segments.len();
        let total_right_len: usize = self
            .right_segments
            .iter()
            .map(|s| s.text.trim().chars().count())
            .sum::<usize>()
            + if right_count > 1 {
                (right_count - 1) * 3
            } else {
                0
            };

        let right_start = max_x.saturating_sub(total_right_len as u16);

        if right_start > x {
            let gap = (right_start - x) as usize;
            if gap > 0 {
                buf.set_stringn(x, bar_y, &"─".repeat(gap), gap, line_style);
            }
            let mut rx = right_start;
            for (idx, seg) in self.right_segments.iter().enumerate() {
                let mut style = Style::default().fg(seg.fg).bg(base_bg);
                if seg.bold {
                    style = style.add_modifier(Modifier::BOLD);
                }
                let text = seg.text.trim();
                let text_len = text.chars().count() as u16;
                if rx < last_x {
                    buf.set_stringn(rx, bar_y, text, (last_x - rx) as usize, style);
                    rx += text_len;
                }
                if idx + 1 < right_count && rx + 3 < last_x {
                    let dot_style = Style::default().fg(p.overlay0).bg(base_bg);
                    buf[(rx, bar_y)].set_symbol(" ").set_style(dot_style);
                    buf[(rx + 1, bar_y)].set_symbol("·").set_style(dot_style);
                    buf[(rx + 2, bar_y)].set_symbol(" ").set_style(dot_style);
                    rx += 3;
                }
            }
            if rx < last_x {
                let tail_gap = (last_x - rx) as usize;
                buf.set_stringn(rx, bar_y, &"─".repeat(tail_gap), tail_gap, line_style);
            }
        } else {
            if x < last_x {
                let tail_gap = (last_x - x) as usize;
                buf.set_stringn(x, bar_y, &"─".repeat(tail_gap), tail_gap, line_style);
            }
        }
    }
}

