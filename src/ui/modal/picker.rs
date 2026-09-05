use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::modal_choice_rows;
use crate::theme::Palette;
use crate::ui::text::{truncate, width};

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

    pub fn is_free(label: &str, value: &str, cat: &str) -> bool {
        cat == "free"
            || label.to_lowercase().contains("free")
            || value.to_lowercase().contains("free")
    }

    pub fn is_commercial(value: &str, cat: &str) -> bool {
        let lower = value.to_lowercase();
        cat == "commercial"
            || lower.starts_with("claude")
            || lower.starts_with("gpt-")
            || lower.starts_with("o1-")
            || lower.starts_with("o3-")
            || lower.starts_with("o4-")
            || lower.starts_with("openai/")
            || lower.starts_with("anthropic/")
            || lower.starts_with("google/")
            || lower.starts_with("cohere/")
            || lower.contains("/claude-")
            || lower.contains("/gpt-")
    }

    pub fn is_open(value: &str, cat: &str) -> bool {
        !Self::is_commercial(value, cat)
    }

    pub fn matches_category(&self, cat_idx: usize, label: &str, value: &str, cat: &str) -> bool {
        match cat_idx {
            1 => Self::is_free(label, value, cat),
            2 => Self::is_open(value, cat),
            3 => Self::is_commercial(value, cat),
            _ => true,
        }
    }

    pub fn category_count(&self, cat_idx: usize) -> usize {
        self.options
            .iter()
            .filter(|(label, value, cat)| self.matches_category(cat_idx, label, value, cat))
            .count()
    }

    pub fn filtered_indices(&self) -> Vec<usize> {
        let q = self.query.trim().to_lowercase();
        self.options
            .iter()
            .enumerate()
            .filter(|(_, (label, value, c))| {
                let cat_ok = self.matches_category(self.category, label, value, c);
                let query_ok = q.is_empty()
                    || label.to_lowercase().contains(&q)
                    || value.to_lowercase().contains(&q)
                    || c.to_lowercase().contains(&q);
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

        if area.height < 4 || area.width < 20 {
            return;
        }

        let [search_area, tabs_area, sep_area, list_area, footer_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .areas::<5>(area);

        let indices = self.filtered_indices();

        let search_style = Style::default().bg(p.surface0);
        let search_icon = format!(" {} ", nf_icons::nf!("nf-cod-search"));
        let mut search_spans = vec![Span::styled(
            search_icon,
            Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
        )];

        if self.query.is_empty() {
            search_spans.push(Span::styled(
                "Search models",
                Style::default().fg(p.overlay0),
            ));
            search_spans.push(Span::styled("█", Style::default().fg(p.accent)));
        } else {
            search_spans.push(Span::styled(
                self.query.clone(),
                Style::default().fg(p.text).add_modifier(Modifier::BOLD),
            ));
            search_spans.push(Span::styled("█", Style::default().fg(p.accent)));
        }

        let used_search_w: usize = search_spans.iter().map(|s| width(&s.content)).sum();
        let count_str = format!("{}/{} ", indices.len(), self.options.len());
        let count_w = width(&count_str);
        let search_pad = (search_area.width as usize).saturating_sub(used_search_w + count_w);
        search_spans.push(Span::raw(" ".repeat(search_pad)));
        search_spans.push(Span::styled(count_str, Style::default().fg(p.overlay1)));
        frame.render_widget(
            Paragraph::new(Line::from(search_spans)).style(search_style),
            search_area,
        );

        let category_defs = [
            (nf_icons::nf!("nf-cod-layers"), "All"),
            (nf_icons::nf!("nf-cod-gift"), "Free"),
            (nf_icons::nf!("nf-cod-code"), "Open Source"),
            (nf_icons::nf!("nf-cod-cloud"), "Commercial"),
        ];

        let mut tabs_spans: Vec<Span> = Vec::new();
        for (i, (icon, label)) in category_defs.iter().enumerate() {
            let active = i == self.category;
            let count = self.category_count(i);
            let tab_str = format!(" {} {} ({}) ", icon, label, count);
            let style = if active {
                Style::default()
                    .fg(p.panel_bg)
                    .bg(p.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(p.overlay1).bg(p.surface0)
            };
            tabs_spans.push(Span::styled(tab_str, style));
            tabs_spans.push(Span::raw(" "));
        }
        frame.render_widget(
            Paragraph::new(Line::from(tabs_spans)).style(Style::default().bg(p.panel_bg)),
            tabs_area,
        );

        let sep_text = " Models ";
        let sep_w = width(sep_text);
        let total_sep_w = sep_area.width as usize;
        let right_dash_w = total_sep_w.saturating_sub(sep_w + 3);
        let sep_spans = vec![
            Span::styled("──", Style::default().fg(p.surface1)),
            Span::styled(
                sep_text,
                Style::default().fg(p.overlay1).add_modifier(Modifier::BOLD),
            ),
            Span::styled("─".repeat(right_dash_w), Style::default().fg(p.surface1)),
        ];
        frame.render_widget(Paragraph::new(Line::from(sep_spans)), sep_area);

        if indices.is_empty() {
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "No models match current filter",
                        Style::default().fg(p.overlay0),
                    ),
                ])),
                list_area,
            );
        } else {
            let visible_count = (list_area.height as usize).max(1);
            let scroll_offset = if self.selected < visible_count {
                0
            } else {
                self.selected.saturating_sub(visible_count - 1)
            };

            let visible_indices =
                &indices[scroll_offset..indices.len().min(scroll_offset + visible_count)];
            let row_rects = modal_choice_rows(list_area, visible_indices.len(), 1);

            let row_w = list_area.width as usize;
            let target_w = row_w.saturating_sub(1);

            for (v_idx, opt_idx) in visible_indices.iter().enumerate() {
                let Some(rect) = row_rects.get(v_idx) else {
                    break;
                };
                let row_idx = scroll_offset + v_idx;
                let (label, value, cat) = &self.options[*opt_idx];
                let is_active = *value == self.current_value
                    || (value.is_empty() && self.current_value.is_empty());
                let is_selected = row_idx == self.selected;

                let row_bg = if is_selected {
                    p.surface0
                } else {
                    p.panel_bg
                };

                let cursor_span = if is_selected {
                    Span::styled(
                        "❯ ",
                        Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::raw("  ")
                };

                let free = Self::is_free(label, value, cat);
                let comm = Self::is_commercial(value, cat);
                let open = !comm;

                let icon_span = if free {
                    Span::styled(
                        format!("{} ", nf_icons::nf!("nf-cod-gift")),
                        Style::default().fg(p.yellow),
                    )
                } else if open {
                    Span::styled(
                        format!("{} ", nf_icons::nf!("nf-cod-code")),
                        Style::default().fg(p.green),
                    )
                } else {
                    Span::styled(
                        format!("{} ", nf_icons::nf!("nf-cod-cloud")),
                        Style::default().fg(p.blue),
                    )
                };

                let tag_span = if free {
                    Span::styled(
                        " FREE ",
                        Style::default()
                            .fg(p.panel_bg)
                            .bg(p.green)
                            .add_modifier(Modifier::BOLD),
                    )
                } else if open {
                    Span::styled(
                        " OPEN ",
                        Style::default()
                            .fg(p.green)
                            .bg(p.surface0)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled(
                        " COMM ",
                        Style::default()
                            .fg(p.blue)
                            .bg(p.surface0)
                            .add_modifier(Modifier::BOLD),
                    )
                };

                let check_span = if is_active {
                    Span::styled(
                        " ✓ ",
                        Style::default()
                            .fg(p.panel_bg)
                            .bg(p.accent)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::raw("   ")
                };

                let right_reserved_w = 6 + 3 + 1;
                let left_reserved_w = 2 + 2;
                let middle_max_w = target_w.saturating_sub(left_reserved_w + right_reserved_w);

                let (vendor_str, model_str) = if let Some((vendor, base)) = value.split_once('/') {
                    (format!("{} / ", vendor), base.to_string())
                } else {
                    (String::new(), label.clone())
                };

                let vendor_w = width(&vendor_str);
                let model_max_w = middle_max_w.saturating_sub(vendor_w);
                let display_model = truncate(&model_str, model_max_w);
                let display_model_w = width(&display_model);
                let used_middle_w = vendor_w + display_model_w;
                let pad_w = middle_max_w.saturating_sub(used_middle_w);

                let model_style = if is_selected {
                    Style::default().fg(p.text).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(p.text)
                };

                let row_spans = vec![
                    cursor_span,
                    icon_span,
                    Span::styled(vendor_str, Style::default().fg(p.overlay0)),
                    Span::styled(display_model, model_style),
                    Span::raw(" ".repeat(pad_w)),
                    tag_span,
                    check_span,
                    Span::raw(" "),
                ];

                frame.render_widget(
                    Paragraph::new(Line::from(row_spans)).style(Style::default().bg(row_bg)),
                    *rect,
                );
            }
        }

        let footer_spans = vec![
            Span::raw(" "),
            Span::styled("[↑↓]", Style::default().fg(p.overlay1)),
            Span::styled(" Move  ", Style::default().fg(p.overlay0)),
            Span::styled("[Tab]", Style::default().fg(p.overlay1)),
            Span::styled(" Category  ", Style::default().fg(p.overlay0)),
            Span::styled("[Enter]", Style::default().fg(p.overlay1)),
            Span::styled(" Select", Style::default().fg(p.overlay0)),
        ];
        frame.render_widget(
            Paragraph::new(Line::from(footer_spans)).style(Style::default().bg(p.panel_bg)),
            footer_area,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_options() -> Vec<(String, String, String)> {
        vec![
            (
                "Qwen3.8-Max".into(),
                "Qwen/Qwen3.8-Max".into(),
                "open".into(),
            ),
            (
                "deepseek-v4-pro".into(),
                "deepseek/deepseek-v4-pro".into(),
                "open".into(),
            ),
            (
                "Laguna S-2.1".into(),
                "poolside/laguna-s-2.1-free".into(),
                "open".into(),
            ),
            (
                "claude-opus-5".into(),
                "anthropic/claude-opus-5".into(),
                "commercial".into(),
            ),
            (
                "gpt-5.6-sol".into(),
                "openai/gpt-5.6-sol".into(),
                "commercial".into(),
            ),
        ]
    }

    #[test]
    fn picker_categorization_filters_correctly() {
        let mut picker = ModelPicker::new(sample_options(), "Qwen/Qwen3.8-Max".into());

        assert_eq!(picker.filtered_indices().len(), 5);

        picker.set_category(1);
        let free_indices = picker.filtered_indices();
        assert_eq!(free_indices.len(), 1);
        assert_eq!(picker.options[free_indices[0]].1, "poolside/laguna-s-2.1-free");

        picker.set_category(2);
        let open_indices = picker.filtered_indices();
        assert_eq!(open_indices.len(), 3);

        picker.set_category(3);
        let comm_indices = picker.filtered_indices();
        assert_eq!(comm_indices.len(), 2);
    }

    #[test]
    fn picker_search_query_filters_across_fields() {
        let mut picker = ModelPicker::new(sample_options(), String::new());

        picker.query = "qwen".into();
        assert_eq!(picker.filtered_indices().len(), 1);

        picker.query = "anthropic".into();
        assert_eq!(picker.filtered_indices().len(), 1);

        picker.query = "nonexistent".into();
        assert_eq!(picker.filtered_indices().len(), 0);
    }

    #[test]
    fn page_move_jumps_by_page_height() {
        let options: Vec<(String, String, String)> = (0..40)
            .map(|i| (format!("Model {i}"), format!("m{i}"), "open".into()))
            .collect();
        let mut picker = ModelPicker::new(options, String::new());
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
    fn renders_picker_without_overflow() {
        use ratatui::backend::TestBackend;

        let mut picker = ModelPicker::new(sample_options(), "Qwen/Qwen3.8-Max".into());
        let p = Palette::dark();
        let backend = TestBackend::new(80, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = f.area();
                picker.render(f, area, &p);
            })
            .expect("picker render must succeed");

        picker.category = 2;
        picker.selected = 1;
        terminal
            .draw(|f| {
                let area = f.area();
                picker.render(f, area, &p);
            })
            .expect("open category render must succeed");
    }
}
