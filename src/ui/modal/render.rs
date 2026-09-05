use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;

use crate::theme::Palette;
use crate::ui::text::width;
use crate::ui::widget::render_modal_shell;

use super::model::{Modal, ModalRow};
pub use super::row_render::{row_content_width, row_spans, row_wrapped_lines};

pub fn dim_background(frame: &mut Frame, area: Rect) {
    let buf = frame.buffer_mut();
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let cell = &mut buf[(x, y)];
            cell.set_style(cell.style().add_modifier(Modifier::DIM));
        }
    }
}

pub fn modal_choice_rows(area: Rect, count: usize, row_height: u16) -> Vec<Rect> {
    let mut rows = Vec::with_capacity(count);
    let mut y = area.y;
    for _ in 0..count {
        if y >= area.y + area.height {
            break;
        }
        let remaining = area.y + area.height - y;
        let height = row_height.min(remaining);
        rows.push(Rect::new(area.x, y, area.width, height));
        y = y.saturating_add(row_height);
    }
    rows
}

pub fn modal_stack_areas(
    inner: Rect,
    header_height: u16,
    actions_height: u16,
    gap: u16,
) -> (Rect, Rect, Option<Rect>) {
    let mut constraints = vec![
        Constraint::Length(header_height),
        Constraint::Length(gap),
        Constraint::Min(0),
    ];
    if actions_height > 0 {
        constraints.push(Constraint::Length(gap));
        constraints.push(Constraint::Length(actions_height));
    }
    let areas = Layout::vertical(constraints).split(inner);
    let header = areas[0];
    let content = areas[2];
    let actions = if actions_height > 0 {
        Some(areas[4])
    } else {
        None
    };
    (header, content, actions)
}

pub fn modal_rect(area: Rect, rows: usize, cmds: usize, content_width: u16) -> Option<Rect> {
    let cmds = cmds.min(8);
    let avail_h = area.height.saturating_sub(2);
    let avail_w = area.width.saturating_sub(4);
    let height = (rows as u16 + 7 + cmds as u16 + 1).clamp(4, avail_h.max(4));
    let width = (content_width + 4).clamp(56, avail_w.min(76).max(56));
    crate::ui::widget::centered_popup_rect(area, width, height)
}

pub fn render_modal(frame: &mut Frame, area: Rect, modal: &Modal, p: &Palette) {
    dim_background(frame, area);

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let spinner_frame = ["|", "/", "-", "\\"][(now_ms as usize / 120) % 4];
    let shown_rows = if modal.page_size > 0 {
        modal
            .visible_rows()
            .iter()
            .map(|r| row_wrapped_lines(r, 68).max(1))
            .sum::<u16>() as usize
    } else {
        modal
            .rows
            .iter()
            .map(|r| row_wrapped_lines(r, 68).max(1))
            .sum::<u16>() as usize
    };

    let tabs_width: u16 = if !modal.steps.is_empty() {
        modal
            .steps
            .iter()
            .map(|s| (width(&s.title) + 4) as u16)
            .sum::<u16>()
            .saturating_add(4)
    } else {
        0
    };
    let row_max = modal
        .visible_rows()
        .iter()
        .map(row_content_width)
        .max()
        .unwrap_or(56);
    let content_width = if !modal.steps.is_empty() {
        56
    } else {
        row_max.max(tabs_width.min(72)).clamp(56, 72)
    };
    let cmds_count = if modal.steps.is_empty() { modal.commands.len() } else { 0 };
    let Some(popup) = modal_rect(area, shown_rows, cmds_count, content_width) else {
        return;
    };
    let popup_w = popup.width;
    let height = popup.height;
    let Some(inner) = render_modal_shell(frame, area, popup_w, height, p) else {
        return;
    };
    if inner.height < 4 {
        return;
    }

    {
        let title_fmt = if !modal.steps.is_empty() {
            format!(" {} {} ", nf_icons::nf!("nf-cod-gear"), modal.title)
        } else {
            format!(" {} ", modal.title)
        };
        let border_blue = Palette::dark().blue;
        let border_style = Style::default()
            .fg(border_blue)
            .bg(p.sidebar_bg)
            .add_modifier(Modifier::BOLD);

        let title_w = width(&title_fmt) as u16;
        let start_x = popup
            .x
            .saturating_add(popup.width.saturating_sub(title_w) / 2);
        for (i, ch) in title_fmt.chars().enumerate() {
            let cx = start_x + i as u16;
            if cx < popup.x + popup.width - 2 {
                frame.buffer_mut()[(cx, popup.y)]
                    .set_symbol(&ch.to_string())
                    .set_style(border_style);
            }
        }

        if !modal.steps.is_empty() {
            let step_badge = format!(" Step {}/{} ", modal.current_step + 1, modal.steps.len());
            let badge_w = width(&step_badge) as u16;
            let badge_x = popup.x + popup.width.saturating_sub(badge_w + 2);
            let badge_style = Style::default()
                .fg(p.accent)
                .bg(p.sidebar_bg)
                .add_modifier(Modifier::BOLD);
            for (i, ch) in step_badge.chars().enumerate() {
                let bx = badge_x + i as u16;
                if bx < popup.x + popup.width - 1 {
                    frame.buffer_mut()[(bx, popup.y)]
                        .set_symbol(&ch.to_string())
                        .set_style(badge_style);
                }
            }
        }
    }

    let has_steps = !modal.steps.is_empty();
    let has_actions = has_steps
        || modal.rows.iter().any(|r| r.is_selectable())
        || !modal.commands.is_empty();
    let (header, content, actions) = if has_steps {
        modal_stack_areas(inner, 1, 1, 1)
    } else if has_actions && inner.height >= 8 {
        modal_stack_areas(inner, 0, 1, 1)
    } else {
        modal_stack_areas(inner, 0, 0, 1)
    };

    if has_steps {
        let n_steps = modal.steps.len();
        let total_w = header.width as usize;

        let tab_natural_widths: Vec<usize> = modal
            .steps
            .iter()
            .map(|s| width(&s.title) + 4)
            .collect();
        let sum_natural_w: usize = tab_natural_widths.iter().sum();

        let mut spans: Vec<Span> = Vec::new();

        if sum_natural_w <= total_w && n_steps > 0 {
            let base_seg_w = total_w / n_steps;
            let remainder = total_w % n_steps;

            for (idx, step) in modal.steps.iter().enumerate() {
                let seg_w = base_seg_w + if idx == n_steps - 1 { remainder } else { 0 };
                let is_active = idx == modal.current_step;
                let style = if is_active {
                    Style::default()
                        .fg(p.panel_bg)
                        .bg(p.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(p.subtext0).bg(p.surface0)
                };

                let label = format!(" {} ", step.title);
                let label_len = width(&label);
                let left_pad = seg_w.saturating_sub(label_len) / 2;
                let right_pad = seg_w.saturating_sub(label_len + left_pad);
                let padded_text = format!("{}{}{}", " ".repeat(left_pad), label, " ".repeat(right_pad));
                spans.push(Span::styled(padded_text, style));
            }
        } else if n_steps > 0 {
            let cur = modal.current_step;
            let mut start_idx = cur;
            let mut end_idx = cur + 1;
            let mut used_w = tab_natural_widths[cur];

            loop {
                let mut expanded = false;
                if end_idx < n_steps {
                    let next_w = tab_natural_widths[end_idx] + if end_idx + 1 < n_steps { 4 } else { 0 };
                    if used_w + next_w <= total_w {
                        used_w += next_w;
                        end_idx += 1;
                        expanded = true;
                    }
                }
                if start_idx > 0 {
                    let prev_w = tab_natural_widths[start_idx - 1] + if start_idx > 1 { 4 } else { 0 };
                    if used_w + prev_w <= total_w {
                        used_w += prev_w;
                        start_idx -= 1;
                        expanded = true;
                    }
                }
                if !expanded {
                    break;
                }
            }

            if start_idx > 0 {
                spans.push(Span::styled(
                    format!("‹+{} ", start_idx),
                    Style::default().fg(p.overlay0).bg(p.surface0),
                ));
            }

            let num_visible = end_idx - start_idx;
            let remaining_w = total_w
                .saturating_sub(if start_idx > 0 { 4 } else { 0 })
                .saturating_sub(if end_idx < n_steps { 4 } else { 0 });
            let base_seg_w = remaining_w / num_visible.max(1);
            let rem = remaining_w % num_visible.max(1);

            for (i, idx) in (start_idx..end_idx).enumerate() {
                let step = &modal.steps[idx];
                let seg_w = base_seg_w + if i == num_visible - 1 { rem } else { 0 };
                let is_active = idx == modal.current_step;
                let style = if is_active {
                    Style::default()
                        .fg(p.panel_bg)
                        .bg(p.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(p.subtext0).bg(p.surface0)
                };

                let label = format!(" {} ", step.title);
                let label_len = width(&label);
                let left_pad = seg_w.saturating_sub(label_len) / 2;
                let right_pad = seg_w.saturating_sub(label_len + left_pad);
                let padded_text = format!("{}{}{}", " ".repeat(left_pad), label, " ".repeat(right_pad));
                spans.push(Span::styled(padded_text, style));
            }

            if end_idx < n_steps {
                spans.push(Span::styled(
                    format!(" +{}›", n_steps - end_idx),
                    Style::default().fg(p.overlay0).bg(p.surface0),
                ));
            }
        }

        frame.render_widget(Paragraph::new(Line::from(spans)), header);

        let sep_y = header.bottom();
        if sep_y < inner.bottom() {
            frame.buffer_mut().set_line(
                inner.left(),
                sep_y,
                &Line::from(vec![Span::styled(
                    "─".repeat(inner.width as usize),
                    Style::default().fg(p.surface1),
                )]),
                inner.width,
            );
        }
    }

    let sticky_h = modal.sticky_footer.len() as u16;
    let (rows_area, sticky_area) = if sticky_h > 0 && content.height > sticky_h + 1 {
        let parts = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(sticky_h),
        ])
        .split(content);
        (parts[0], Some((parts[1], parts[2])))
    } else {
        (content, None)
    };

    let visible: Vec<&ModalRow> = modal.visible_rows().iter().collect();
    let wrap_w = rows_area.width as usize;

    let row_heights: Vec<u16> = visible
        .iter()
        .map(|row| row_wrapped_lines(row, wrap_w).max(1))
        .collect();
    let mut y = rows_area.y;
    for (idx, (row, &height)) in visible.iter().zip(row_heights.iter()).enumerate() {
        if y >= rows_area.bottom() {
            break;
        }
        let abs_idx = modal.page_start() + idx;
        let is_selected = abs_idx == modal.selected;
        let style = if is_selected {
            Style::default()
                .bg(p.surface0)
                .fg(p.text)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.subtext0)
        };
        let rect = Rect::new(rows_area.x, y, rows_area.width, height.min(rows_area.bottom() - y));
        let spans = row_spans(row, p, is_selected, wrap_w, spinner_frame);
        frame.render_widget(
            Paragraph::new(Line::from(spans))
                .style(style)
                .wrap(Wrap { trim: false }),
            rect,
        );
        y += height;
    }

    if let Some((sep_rect, sticky_rect)) = sticky_area {
        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                "─".repeat(sticky_rect.width as usize),
                Style::default().fg(p.surface1),
            )])),
            sep_rect,
        );
        for (i, row) in modal.sticky_footer.iter().enumerate() {
            let rect = Rect::new(
                sticky_rect.x,
                sticky_rect.y + i as u16,
                sticky_rect.width,
                1,
            );
            let spans = row_spans(row, p, false, sticky_rect.width as usize, spinner_frame);
            frame.render_widget(
                Paragraph::new(Line::from(spans)).style(Style::default().fg(p.text)),
                rect,
            );
        }
    }

    if !modal.commands.is_empty() && modal.steps.is_empty() {
        let max_cmds = 8usize;
        let shown = modal.commands.len().min(max_cmds);
        let cmds_top = rows_area.y + rows_area.height.saturating_sub(shown as u16 + 1);
        let rule_rect = Rect::new(rows_area.x, cmds_top, rows_area.width, 1);
        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                "─".repeat(rows_area.width as usize),
                Style::default().fg(p.surface1),
            )])),
            rule_rect,
        );
        for (i, (name, desc)) in modal.commands.iter().take(shown).enumerate() {
            let idx = modal.rows.len() + i;
            let is_selected = idx == modal.selected;
            let rect = Rect::new(rows_area.x, cmds_top + 1 + i as u16, rows_area.width, 1);
            let style = if is_selected {
                Style::default()
                    .bg(p.surface0)
                    .fg(p.text)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(p.subtext0)
            };
            let line = Line::from(vec![
                Span::styled(
                    if is_selected { "▶ " } else { "  " },
                    Style::default().fg(if is_selected { p.accent } else { p.overlay0 }),
                ),
                Span::styled(format!("/{name}"), Style::default().fg(p.accent)),
                Span::styled(format!("  {desc}"), Style::default().fg(p.overlay1)),
            ]);

            let line = crate::ui::widget::truncate_line(&line, rect.width as usize);
            frame.render_widget(Paragraph::new(line).style(style), rect);
        }
        if modal.commands.len() > max_cmds {
            let rect = Rect::new(
                rows_area.x,
                cmds_top + 1 + max_cmds as u16,
                rows_area.width,
                1,
            );
            frame.render_widget(
                Paragraph::new(Line::from(vec![Span::styled(
                    format!("  +{} more", modal.commands.len() - max_cmds),
                    Style::default().fg(p.overlay0),
                )])),
                rect,
            );
        }
    }

    if let Some(actions_rect) = actions {
        let rule_rect = Rect::new(
            actions_rect.x,
            actions_rect.y.saturating_sub(1),
            actions_rect.width,
            1,
        );
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "─".repeat(actions_rect.width as usize),
                Style::default().fg(p.surface1),
            ))),
            rule_rect,
        );

        let mut pairs: Vec<(&str, String)> = Vec::new();

        let has_selectable = modal.rows.iter().any(|r| r.is_selectable()) || !modal.commands.is_empty();
        if has_selectable {
            pairs.push(("↑↓", "Move".to_string()));
        }

        if !modal.steps.is_empty() {
            pairs.push(("Tab", "Step".to_string()));
        }

        let current_row = modal.rows.get(modal.selected);
        if matches!(
            current_row,
            Some(ModalRow::Stepper { .. }) | Some(ModalRow::Choice { .. })
        ) {
            pairs.push(("←→", "Adjust".to_string()));
        }

        if modal.editing_text {
            pairs.push(("Enter", "Commit".to_string()));
        } else if matches!(current_row, Some(ModalRow::TextInput { .. })) {
            pairs.push(("Enter", "Edit".to_string()));
        } else if matches!(current_row, Some(ModalRow::Toggle { .. })) {
            pairs.push(("Enter", "Toggle".to_string()));
        } else if matches!(current_row, Some(ModalRow::Choice { .. })) {
            pairs.push(("Enter", "Select".to_string()));
        } else if matches!(current_row, Some(ModalRow::Nav { .. })) {
            pairs.push(("Enter", "Open".to_string()));
        } else if !modal.commands.is_empty() && modal.selected >= modal.rows.len() {
            pairs.push(("Enter", "Run".to_string()));
        }

        if modal.page_size > 0 && modal.page_count() > 1 {
            pairs.push((
                "^U/^D",
                format!("Page {}/{}", modal.page + 1, modal.page_count()),
            ));
        }

        for (k, v) in &modal.hints {
            pairs.push((k.as_str(), v.clone()));
        }

        let esc_label = if modal.editing_text { "Cancel" } else { "Close" };
        pairs.push(("Esc", esc_label.to_string()));

        let mut spans: Vec<Span<'static>> = Vec::new();
        for (i, (key, label)) in pairs.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled("   ", Style::default()));
            }
            spans.push(Span::styled(
                format!(" {key} "),
                Style::default()
                    .fg(p.accent)
                    .bg(p.surface0)
                    .add_modifier(Modifier::BOLD),
            ));
            if !label.is_empty() {
                spans.push(Span::styled(
                    format!(" {label}"),
                    Style::default().fg(p.subtext0),
                ));
            }
        }
        let mut line = Line::from(spans);
        let hint_w = actions_rect.width as usize;
        if line.width() > hint_w {
            line = crate::ui::widget::truncate_line(&line, hint_w);
        }
        frame.render_widget(Paragraph::new(line), actions_rect);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modal_content_width_is_bounded_and_never_bloats() {
        let area = Rect::new(0, 0, 140, 40);
        let popup = modal_rect(area, 10, 0, 72).unwrap();
        assert!(popup.width <= 76);
        let popup_wide = modal_rect(area, 10, 0, 120).unwrap();
        assert!(popup_wide.width <= 76);
    }
}
