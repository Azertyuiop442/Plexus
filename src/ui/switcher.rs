
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::theme::Palette;
use crate::ui::search::fuzzy::fuzzy_match;

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SwitcherAction {
    SwitchTab(usize),
    ResumeSession(String),
    ExecuteSlashCommand(String),
    OpenPreferences,
    OpenModConfig(usize),
    OpenContext,
}

#[derive(Debug, Clone)]
pub struct SwitcherItem {
    pub title: String,
    pub subtitle: String,
    pub icon: &'static str,
    pub action: SwitcherAction,
}

#[derive(Debug, Clone)]
pub struct SwitcherState {
    pub query: String,
    pub selected: usize,
    pub scroll: usize,
    pub items: Vec<SwitcherItem>,
    pub filtered: Vec<(usize, i64)>,
}

impl SwitcherState {
    pub fn new(items: Vec<SwitcherItem>) -> Self {
        let mut s = Self {
            query: String::new(),
            selected: 0,
            scroll: 0,
            items,
            filtered: Vec::new(),
        };
        s.refilter();
        s
    }

    pub fn refilter(&mut self) {
        self.filtered.clear();
        let q = self.query.trim();

        if q.is_empty() {
            self.filtered = (0..self.items.len()).map(|i| (i, 0)).collect();
        } else {
            for (idx, item) in self.items.iter().enumerate() {
                let search_target = format!("{} {}", item.title, item.subtitle);
                if let Some(m) = fuzzy_match(q, &search_target) {
                    self.filtered.push((idx, m.score));
                }
            }

            self.filtered.sort_by(|a, b| b.1.cmp(&a.1));
        }

        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.query.push(c);
        self.refilter();
    }

    pub fn backspace(&mut self) {
        self.query.pop();
        self.refilter();
    }

    pub fn next(&mut self) {
        if !self.filtered.is_empty() && self.selected + 1 < self.filtered.len() {
            self.selected += 1;
        }
    }

    pub fn prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn selected_action(&self) -> Option<SwitcherAction> {
        let (item_idx, _) = self.filtered.get(self.selected)?;
        self.items.get(*item_idx).map(|item| item.action.clone())
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, p: &Palette) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let w = area.width.saturating_sub(10).clamp(50, 90).min(area.width);
        let h = 16u16.min(area.height.saturating_sub(4));

        let x = area.x + (area.width.saturating_sub(w)) / 2;
        let y = area.y + (area.height.saturating_sub(h)) / 3;
        let popup = Rect::new(x, y, w, h);

        frame.render_widget(Clear, popup);
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(p.accent).bg(p.surface0))
            .style(Style::new().bg(p.surface0));
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        if inner.height < 4 {
            return;
        }

        let prompt = " 🔍 ";
        let cursor_char = "█";
        let query_spans = vec![
            Span::styled(prompt, Style::new().fg(p.accent).bold()),
            Span::styled(&self.query, Style::new().fg(p.text).bold()),
            Span::styled(cursor_char, Style::new().fg(p.accent)),
        ];
        frame.render_widget(Paragraph::new(Line::from(query_spans)), Rect::new(inner.x, inner.y, inner.width, 1));

        let sep = "─".repeat(inner.width as usize);
        frame.render_widget(
            Paragraph::new(Span::styled(sep, Style::new().fg(p.surface1))),
            Rect::new(inner.x, inner.y + 1, inner.width, 1),
        );

        let list_y = inner.y + 2;
        let list_h = inner.height.saturating_sub(3);
        let cap = list_h as usize;

        if self.filtered.is_empty() {
            frame.render_widget(
                Paragraph::new(Span::styled(" No matching items", Style::new().fg(p.overlay0))),
                Rect::new(inner.x + 2, list_y, inner.width.saturating_sub(4), 1),
            );
        } else {
            let max_scroll = self.filtered.len().saturating_sub(cap);
            let scroll = if self.selected >= self.scroll + cap {
                self.selected.saturating_sub(cap - 1)
            } else if self.selected < self.scroll {
                self.selected
            } else {
                self.scroll
            }
            .min(max_scroll);

            for (slot, &(item_idx, _)) in self.filtered.iter().skip(scroll).take(cap).enumerate() {
                let actual_idx = scroll + slot;
                let is_selected = actual_idx == self.selected;
                let cur_y = list_y + slot as u16;

                if let Some(item) = self.items.get(item_idx) {
                    let icon_span = Span::styled(
                        format!(" {} ", item.icon),
                        Style::new().fg(if is_selected { p.panel_bg } else { p.accent }),
                    );
                    let title_span = Span::styled(
                        format!("{:<24}", item.title),
                        Style::new().fg(if is_selected { p.panel_bg } else { p.text }).bold(),
                    );
                    let sub_span = Span::styled(
                        format!(" {}", item.subtitle),
                        Style::new().fg(if is_selected { p.surface0 } else { p.subtext0 }),
                    );

                    let bg = if is_selected { p.accent } else { p.surface0 };
                    let row_line = Line::from(vec![icon_span, title_span, sub_span]);

                    let row_block = Block::new().style(Style::new().bg(bg));
                    let row_rect = Rect::new(inner.x, cur_y, inner.width, 1);
                    frame.render_widget(row_block, row_rect);
                    frame.render_widget(Paragraph::new(row_line), row_rect);
                }
            }
        }

        let footer_y = inner.bottom().saturating_sub(1);
        frame.render_widget(
            Paragraph::new(Span::styled(
                "  ↑↓ Navigate · ⏎ Select · Esc Cancel",
                Style::new().fg(p.subtext0),
            )),
            Rect::new(inner.x, footer_y, inner.width, 1),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_switcher_filtering() {
        let items = vec![
            SwitcherItem {
                title: "Terminal 1".into(),
                subtitle: "Active workspace".into(),
                icon: "⧉",
                action: SwitcherAction::SwitchTab(0),
            },
            SwitcherItem {
                title: "Example Run".into(),
                subtitle: "Launch MoA Pipeline".into(),
                icon: "⚡",
                action: SwitcherAction::ExecuteSlashCommand("example".into()),
            },
        ];

        let mut switcher = SwitcherState::new(items);
        assert_eq!(switcher.filtered.len(), 2);

        switcher.insert_char('e');
        switcher.insert_char('x');

        assert_eq!(switcher.filtered.len(), 1);
        assert_eq!(
            switcher.selected_action(),
            Some(SwitcherAction::ExecuteSlashCommand("example".into()))
        );
    }
}

