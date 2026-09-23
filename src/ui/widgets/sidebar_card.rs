
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::Frame;

use crate::theme::Palette;

pub struct SidebarCardWidget {
    pub title: String,
    pub icon: String,
    pub lines: Vec<(String, String)>,
}

impl SidebarCardWidget {
    pub fn new(title: impl Into<String>, icon: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            icon: icon.into(),
            lines: Vec::new(),
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, p: &Palette) {
        let buf = frame.buffer_mut();
        let bg = p.sidebar_bg;

        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)]
                    .set_symbol(" ")
                    .set_style(Style::default().bg(bg));
            }
        }

        let header = format!(" {} {} ", self.icon, self.title);
        if area.height >= 1 {
            for (i, ch) in header.chars().enumerate() {
                let cx = area.left() + i as u16;
                if cx < area.right() {
                    buf[(cx, area.top())].set_symbol(&ch.to_string()).set_style(
                        Style::default()
                            .fg(p.accent)
                            .bg(bg)
                            .add_modifier(Modifier::BOLD),
                    );
                }
            }
        }

        for (row_idx, (label, val)) in self.lines.iter().enumerate() {
            let y = area.top() + 1 + row_idx as u16;
            if y < area.bottom() {
                let line_str = format!("  {}: {}", label, val);
                for (i, ch) in line_str.chars().enumerate() {
                    let cx = area.left() + i as u16;
                    if cx < area.right() {
                        buf[(cx, y)]
                            .set_symbol(&ch.to_string())
                            .set_style(Style::default().fg(p.subtext0).bg(bg));
                    }
                }
            }
        }
    }
}

