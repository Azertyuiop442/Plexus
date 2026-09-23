use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use crate::theme::Palette;

pub fn render_collapsed_sidebar(frame: &mut ratatui::Frame, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let p = Palette::dark();
    let bg = crate::theme::effective_bg();
    let x = area.x;
    let mid_y = area.y + area.height / 2;
    let border_style = Style::default().fg(p.surface1);
    let grip_style = Style::default().fg(p.accent).add_modifier(Modifier::BOLD);

    for y in area.y..area.y + area.height {
        let is_grip = y >= mid_y.saturating_sub(1) && y <= mid_y + 1;
        let (sym, style) = if is_grip {
            ("║", grip_style)
        } else {
            ("│", border_style)
        };
        let cell = &mut frame.buffer_mut()[(x, y)];
        cell.set_symbol(sym);
        cell.set_style(style);
        cell.set_bg(bg);
    }
}
