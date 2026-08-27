use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::modal_choice_rows;
use crate::theme::Palette;

pub const PICKER_CATEGORIES: [&str; 4] = ["all", "free", "open", "commercial"];

#[derive(Debug, Clone)]
pub struct ModelPicker {

    pub options: Vec<(String, String, String)>,

    pub current_value: String,

    pub query: String,

    pub category: usize,

    pub selected: usize,
}

impl ModelPicker {
    pub fn new(options: Vec<(String, String, String)>, current_value: String) -> Self {
        Self {
            options,
            current_value,
            query: String::new(),
            category: 0,
            selected: 0,
        }
    }

    pub fn filtered_indices(&self) -> Vec<usize> {
        let cat = PICKER_CATEGORIES[self.category];
        self.options
            .iter()
            .enumerate()
            .filter(|(_, (label, value, c))| {
                let is_free = label.to_lowercase().contains("free")
                    || value.to_lowercase().contains("free")
                    || c == "free";
                let cat_ok = match cat {
                    "all" => true,
                    "free" => is_free,
                    _ => c == cat,
                };
                let q = self.query.trim().to_lowercase();
                let query_ok = q.is_empty()
                    || label.to_lowercase().contains(&q)
                    || value.to_lowercase().contains(&q);
                cat_ok && query_ok
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub fn cycle_category(&mut self) {
        self.category = (self.category + 1) % PICKER_CATEGORIES.len();
        self.selected = 0;
    }

    pub fn set_category(&mut self, idx: usize) {
        if idx < PICKER_CATEGORIES.len() {
            self.category = idx;
            self.selected = 0;
        }
    }

    pub fn move_selection(&mut self, delta: isize) {
        let n = self.filtered_indices().len();
        if n == 0 {
            return;
        }
        self.selected = (self.selected as isize + delta).rem_euclid(n as isize) as usize;
    }

    pub fn page_move(&mut self, delta: isize, page_height: usize) {
        let n = self.filtered_indices().len();
        if n == 0 {
            return;
        }
        let step = page_height.max(1) as isize;
        self.selected = (self.selected as isize + delta * step).clamp(0, n as isize - 1) as usize;
    }

    pub fn highlighted_option(&self) -> Option<usize> {
        self.filtered_indices().get(self.selected).copied()
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, p: &Palette) {
        let buf = frame.buffer_mut();
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)]
                    .set_symbol(" ")
                    .set_style(Style::default().bg(p.panel_bg));
            }
        }

        let [search_area, chips_area, list_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .areas::<3>(area);

        let search_style = Style::default().fg(p.text).bg(p.surface0);
        let search_line = Line::from(vec![
            Span::styled(
                "/",
                Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" {}", self.query), Style::default().fg(p.text)),
            Span::styled("█", Style::default().fg(p.accent)),
        ]);
        frame.render_widget(Paragraph::new(search_line).style(search_style), search_area);

        let mut chips: Vec<Span> = Vec::new();
        for (i, cat) in PICKER_CATEGORIES.iter().enumerate() {
            let active = i == self.category;
            let style = if active {
                Style::default()
                    .fg(p.panel_bg)
                    .bg(p.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(p.overlay1)
            };
            chips.push(Span::styled(format!(" {} ", cat), style));
            chips.push(Span::raw(" "));
        }
        frame.render_widget(
            Paragraph::new(Line::from(chips)).style(Style::default().bg(p.panel_bg)),
            chips_area,
        );

        let indices = self.filtered_indices();
        if indices.is_empty() {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "no match",
                    Style::default().fg(p.overlay0),
                ))),
                list_area,
            );
            return;
        }

        let visible_count = (list_area.height as usize).max(1);
        let scroll_offset = if self.selected < visible_count {
            0
        } else {
            self.selected.saturating_sub(visible_count - 1)
        };

        let visible_indices =
            &indices[scroll_offset..indices.len().min(scroll_offset + visible_count)];
        let row_rects = modal_choice_rows(list_area, visible_indices.len(), 1);

        for (v_idx, opt_idx) in visible_indices.iter().enumerate() {
            let Some(rect) = row_rects.get(v_idx) else {
                break;
            };
            let row_idx = scroll_offset + v_idx;
            let (label, value, category) = &self.options[*opt_idx];
            let is_active = *value == self.current_value;
            let is_selected = row_idx == self.selected;

            let style = if is_selected {
                Style::default()
                    .bg(p.surface0)
                    .fg(p.text)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(p.subtext0).bg(p.panel_bg)
            };

            let cat_mark = match category.as_str() {
                "open" => "O",
                "commercial" => "C",
                _ => "·",
            };
            let cat_style = match category.as_str() {
                "open" => Style::default().fg(p.green).add_modifier(Modifier::BOLD),
                "commercial" => Style::default().fg(p.blue).add_modifier(Modifier::BOLD),
                _ => Style::default().fg(p.overlay0),
            };

            let is_free = label.to_lowercase().contains("free")
                || value.to_lowercase().contains("free")
                || category == "free";

            let mut spans = vec![
                Span::styled(format!("{} ", cat_mark), cat_style),
                Span::styled(
                    label.clone(),
                    if is_selected {
                        Style::default().fg(p.text).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(p.text)
                    },
                ),
            ];
            if is_free {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    " FREE ",
                    Style::default()
                        .fg(p.panel_bg)
                        .bg(p.green)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            if is_active {
                spans.push(Span::styled(
                    "  ✓",
                    Style::default().fg(p.green).add_modifier(Modifier::BOLD),
                ));
            }
            frame.render_widget(
                ratatui::widgets::Paragraph::new(Line::from(spans)).style(style),
                *rect,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(n: usize) -> Vec<(String, String, String)> {
        (0..n)
            .map(|i| (format!("Model {i}"), format!("m{i}"), "open".into()))
            .collect()
    }

    #[test]
    fn page_move_jumps_by_page_height() {
        let mut picker = ModelPicker::new(options(40), String::new());
        assert_eq!(picker.filtered_indices().len(), 40);

        picker.page_move(1, 14);
        assert_eq!(picker.selected, 14);
        picker.page_move(1, 14);
        assert_eq!(picker.selected, 28);
        picker.page_move(1, 14);
        assert_eq!(picker.selected, 39);
        picker.page_move(-1, 14);
        assert_eq!(picker.selected, 25);
        picker.page_move(-10, 14);
        assert_eq!(picker.selected, 0);
    }

    #[test]
    fn page_move_noop_on_empty_list() {
        let mut picker = ModelPicker::new(vec![], String::new());
        picker.page_move(1, 14);
        assert_eq!(picker.selected, 0);
    }

    #[test]
    fn renders_huge_list_without_panicking() {
        use ratatui::backend::TestBackend;

        let mut picker = ModelPicker::new(options(250), String::new());
        let p = Palette::dark();
        let backend = TestBackend::new(120, 40);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let area = f.area();
                picker.render(f, area, &p);
            })
            .expect("huge picker list must render");

        picker.selected = 200;
        picker.page_move(1, 14);
        assert_eq!(picker.selected, 214);
        picker.page_move(10, 14);
        assert_eq!(picker.selected, 249);
    }
}

