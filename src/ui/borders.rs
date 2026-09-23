
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, BorderType, Borders};
use ratatui::Frame;

use crate::theme::Palette;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum DividerAxis {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HoverDivider {
    pub axis: DividerAxis,
    pub line: u16,
    pub span: (u16, u16),
}

impl HoverDivider {

    #[allow(dead_code)]
    pub fn touches(&self, rect: Rect) -> bool {
        let right = rect.x + rect.width.saturating_sub(1);
        let bottom = rect.y + rect.height.saturating_sub(1);
        let near = |coord: u16, a: u16, b: u16| {
            (coord as i32 - a as i32).abs() <= 1 || (coord as i32 - b as i32).abs() <= 1
        };

        match self.axis {
            DividerAxis::Vertical => {
                near(self.line, rect.x, right) && self.span.0 <= bottom && self.span.1 >= rect.y
            }
            DividerAxis::Horizontal => {
                near(self.line, rect.y, bottom) && self.span.0 <= right && self.span.1 >= rect.x
            }
        }
    }
}

#[allow(dead_code)]
pub fn render_pane_box(
    frame: &mut Frame,
    rect: Rect,
    is_focused: bool,
    is_hovered: bool,
    p: &Palette,
) {
    if rect.width < 2 || rect.height < 2 {
        return;
    }

    let color = if is_focused {
        p.accent
    } else if is_hovered {
        p.overlay1
    } else {
        p.overlay0
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color));

    frame.render_widget(block, rect);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn divider_touch_detection() {
        let rect_left = Rect::new(0, 0, 40, 20);
        let rect_right = Rect::new(40, 0, 40, 20);

        let divider = HoverDivider {
            axis: DividerAxis::Vertical,
            line: 40,
            span: (0, 20),
        };

        assert!(divider.touches(rect_left));
        assert!(divider.touches(rect_right));

        let far_rect = Rect::new(100, 50, 20, 10);
        assert!(!divider.touches(far_rect));
    }
}

