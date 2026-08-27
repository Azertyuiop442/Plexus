
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::theme::Palette;
use crate::ui::text::width as display_width;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextMenuAction {

    TabRename(usize),
    TabClose(usize),
    TabDuplicate(usize),
    TabSplitRight(usize),
    TabSplitDown(usize),

    SessionOpen(String),
    SessionDelete(String),
    SessionCopyId(String),

    PaneInspect(usize),
    PaneClear(usize),
    PaneScrollback(usize),
}

#[derive(Debug, Clone)]
pub struct MenuRow {
    pub text: String,
    pub action: Option<ContextMenuAction>,
    pub divider: bool,
    pub destructive: bool,
}

#[derive(Debug, Clone)]
pub struct ContextMenu {
    pub anchor: (u16, u16),
    pub rows: Vec<MenuRow>,
    pub rects: Vec<Rect>,
    pub popup_rect: Rect,
}

impl ContextMenu {
    pub fn for_tab(tab_idx: usize, title: &str, anchor: (u16, u16)) -> Self {
        let rows = vec![
            MenuRow {
                text: format!("Rename \"{}\"", title),
                action: Some(ContextMenuAction::TabRename(tab_idx)),
                divider: false,
                destructive: false,
            },
            MenuRow {
                text: "Duplicate Tab".into(),
                action: Some(ContextMenuAction::TabDuplicate(tab_idx)),
                divider: false,
                destructive: false,
            },
            MenuRow {
                text: "─".into(),
                action: None,
                divider: true,
                destructive: false,
            },
            MenuRow {
                text: "Split Vertical (Right)".into(),
                action: Some(ContextMenuAction::TabSplitRight(tab_idx)),
                divider: false,
                destructive: false,
            },
            MenuRow {
                text: "Split Horizontal (Down)".into(),
                action: Some(ContextMenuAction::TabSplitDown(tab_idx)),
                divider: false,
                destructive: false,
            },
            MenuRow {
                text: "─".into(),
                action: None,
                divider: true,
                destructive: false,
            },
            MenuRow {
                text: "Close Tab".into(),
                action: Some(ContextMenuAction::TabClose(tab_idx)),
                divider: false,
                destructive: true,
            },
        ];
        Self {
            anchor,
            rows,
            rects: Vec::new(),
            popup_rect: Rect::default(),
        }
    }

    pub fn for_session(sess_id: &str, title: &str, is_open: bool, anchor: (u16, u16)) -> Self {
        let mut rows = Vec::new();
        rows.push(MenuRow {
            text: if is_open {
                format!("Switch to \"{}\"", title)
            } else {
                format!("Resume \"{}\"", title)
            },
            action: Some(ContextMenuAction::SessionOpen(sess_id.to_string())),
            divider: false,
            destructive: false,
        });
        rows.push(MenuRow {
            text: "Copy Session ID".into(),
            action: Some(ContextMenuAction::SessionCopyId(sess_id.to_string())),
            divider: false,
            destructive: false,
        });
        if !is_open {
            rows.push(MenuRow {
                text: "─".into(),
                action: None,
                divider: true,
                destructive: false,
            });
            rows.push(MenuRow {
                text: "Delete Session".into(),
                action: Some(ContextMenuAction::SessionDelete(sess_id.to_string())),
                divider: false,
                destructive: true,
            });
        }
        Self {
            anchor,
            rows,
            rects: Vec::new(),
            popup_rect: Rect::default(),
        }
    }

    pub fn for_pane(pane_idx: usize, anchor: (u16, u16)) -> Self {
        let rows = vec![
            MenuRow {
                text: "Inspect Running Process (Running Now)".into(),
                action: Some(ContextMenuAction::PaneInspect(pane_idx)),
                divider: false,
                destructive: false,
            },
            MenuRow {
                text: "Toggle Scrollback View".into(),
                action: Some(ContextMenuAction::PaneScrollback(pane_idx)),
                divider: false,
                destructive: false,
            },
            MenuRow {
                text: "─".into(),
                action: None,
                divider: true,
                destructive: false,
            },
            MenuRow {
                text: "Clear Screen Buffer".into(),
                action: Some(ContextMenuAction::PaneClear(pane_idx)),
                divider: false,
                destructive: false,
            },
            MenuRow {
                text: "Close Pane".into(),
                action: Some(ContextMenuAction::TabClose(pane_idx)),
                divider: false,
                destructive: true,
            },
        ];
        Self {
            anchor,
            rows,
            rects: Vec::new(),
            popup_rect: Rect::default(),
        }
    }

    pub fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        hover: Option<(u16, u16)>,
        p: &Palette,
    ) {
        if self.rows.is_empty() || area.width == 0 || area.height == 0 {
            return;
        }

        let (ax, ay) = self.anchor;
        let label_w = self
            .rows
            .iter()
            .map(|r| display_width(&r.text))
            .max()
            .unwrap_or(12) as u16;
        let w = (label_w + 4).clamp(16, area.width.max(16));
        let h = (self.rows.len() as u16 + 2).min(area.height.max(2));

        let x = ax.min(area.right().saturating_sub(w)).max(area.x);
        let y = ay.min(area.bottom().saturating_sub(h)).max(area.y);
        let popup = Rect::new(x, y, w, h);
        self.popup_rect = popup;

        frame.render_widget(Clear, popup);
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(p.accent).bg(p.surface0))
            .style(Style::new().bg(p.surface0));
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        self.rects.clear();
        self.rects.reserve(self.rows.len());

        for (i, r) in self.rows.iter().enumerate() {
            if i >= inner.height as usize {
                break;
            }
            let row_rect = Rect::new(inner.x, inner.y + i as u16, inner.width, 1);
            if r.divider {
                let sep = "─".repeat(inner.width as usize);
                frame.render_widget(
                    Paragraph::new(Span::styled(
                        sep,
                        Style::new().fg(p.surface1).bg(p.surface0),
                    )),
                    row_rect,
                );
                self.rects.push(row_rect);
                continue;
            }

            let is_hovered = hover.is_some_and(|(hx, hy)| {
                hx >= row_rect.x && hx < row_rect.right() && hy == row_rect.y
            });

            let fg = if is_hovered {
                p.panel_bg
            } else if r.destructive {
                p.red
            } else {
                p.text
            };
            let bg = if is_hovered {
                if r.destructive {
                    p.red
                } else {
                    p.accent
                }
            } else {
                p.surface0
            };

            let style = if is_hovered {
                Style::new().fg(fg).bg(bg).bold()
            } else {
                Style::new().fg(fg).bg(bg)
            };

            frame.render_widget(
                Paragraph::new(Span::styled(format!(" {}", r.text), style)),
                row_rect,
            );
            self.rects.push(row_rect);
        }
    }

    pub fn hit_test(&self, col: u16, row: u16) -> Option<ContextMenuAction> {
        for (i, rect) in self.rects.iter().enumerate() {
            if col >= rect.x && col < rect.right() && row >= rect.y && row < rect.bottom() {
                if let Some(r) = self.rows.get(i) {
                    if !r.divider {
                        return r.action.clone();
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_menu_for_tab_hit_tests() {
        let mut menu = ContextMenu::for_tab(0, "Terminal 1", (10, 5));
        menu.rects = vec![
            Rect::new(10, 6, 20, 1),
            Rect::new(10, 7, 20, 1),
            Rect::new(10, 8, 20, 1),
            Rect::new(10, 9, 20, 1),
            Rect::new(10, 10, 20, 1),
            Rect::new(10, 11, 20, 1),
            Rect::new(10, 12, 20, 1),
        ];

        assert_eq!(
            menu.hit_test(15, 6),
            Some(ContextMenuAction::TabRename(0))
        );
        assert_eq!(
            menu.hit_test(15, 7),
            Some(ContextMenuAction::TabDuplicate(0))
        );
        assert_eq!(menu.hit_test(15, 8), None);
        assert_eq!(
            menu.hit_test(15, 12),
            Some(ContextMenuAction::TabClose(0))
        );
    }
}

