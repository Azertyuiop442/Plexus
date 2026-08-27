
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;

use crate::theme::Palette;
use crate::ui::mod_bridge::color_from_name;
use crate::ui::text::{truncate, width};
use crate::ui::widget::render_modal_shell;

use super::ansi::ansi_spans;
use super::model::{Modal, ModalRow};

#[allow(dead_code)]
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

pub fn row_content_width(row: &ModalRow) -> u16 {
    let text = match row {
        ModalRow::Toggle { label, .. } => label.clone(),
        ModalRow::Choice { label, options, .. } => {
            let cur = options
                .get(options.len().saturating_sub(1))
                .map(|(d, _, _)| d.clone())
                .unwrap_or_default();
            format!("{label}  {cur}")
        }
        ModalRow::TextInput { label, value, .. } => format!("{label}  {value}"),
        ModalRow::Info(t) => t.clone(),
        ModalRow::InfoColored { text, .. } => text.clone(),
        ModalRow::Separator(t) => t.clone(),
        ModalRow::Progress { label, .. } => format!("{label}  0/0"),
        ModalRow::Stepper { label, value, unit, .. } => format!("{label}  ‹ {value}{unit} ›"),
        ModalRow::Table { headers, rows, .. } => {
            let cols = headers.iter().map(|h| width(h)).max().unwrap_or(0)
                + rows
                    .iter()
                    .flat_map(|r| r.iter().map(|c| width(c)))
                    .max()
                    .unwrap_or(0);
            format!("{}{}", headers.join("  "), "x".repeat(cols))
        }
        ModalRow::Section { title, .. } => title.clone(),
    };

    let visible: usize = text
        .split('\x1b')
        .map(|seg| {
            let seg = seg.strip_prefix('[').unwrap_or(seg);
            let seg = seg.split_once('m').map(|(_, rest)| rest).unwrap_or(seg);
            width(seg)
        })
        .sum();
    (visible as u16).clamp(56, 120)
}

pub fn modal_rect(area: Rect, rows: usize, cmds: usize, content_width: u16) -> Option<Rect> {
    let cmds = cmds.min(8);
    let avail_h = area.height.saturating_sub(2);
    let avail_w = area.width.saturating_sub(4);
    let height = (rows as u16 + 7 + cmds as u16 + 1).clamp(4, avail_h.max(4));
    let width = (content_width + 4).clamp(56, avail_w.max(56));
    crate::ui::widget::centered_popup_rect(area, width, height)
}

pub fn render_modal(frame: &mut Frame, area: Rect, modal: &Modal, p: &Palette) {

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let spinner_frame = ["|", "/", "-", "\\"][(now_ms as usize / 120) % 4];
    let shown_rows = if modal.page_size > 0 {

        modal
            .visible_rows()
            .iter()
            .map(|r| row_wrapped_lines(r, 78).max(1))
            .sum::<u16>() as usize
    } else {
        modal
            .rows
            .iter()
            .map(|r| row_wrapped_lines(r, 78).max(1))
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
    let content_width = row_max.max(tabs_width).clamp(56, 120);
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
            format!(
                " {} ({}) ",
                modal.title, modal.steps[modal.current_step].title
            )
        } else {
            format!(" {} ", modal.title)
        };
        let border_blue = Palette::dark().blue;
        let border_style = Style::default()
            .fg(border_blue)
            .bg(ratatui::style::Color::Rgb(0, 0, 0))
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

    }

    let has_steps = !modal.steps.is_empty();
    let (header, content, actions) = if has_steps {
        modal_stack_areas(inner, 1, 1, 1)
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
                    if used_w + next_w <= total_w.saturating_sub(if start_idx > 0 { 4 } else { 0 }) {
                        used_w += tab_natural_widths[end_idx];
                        end_idx += 1;
                        expanded = true;
                    }
                }
                if start_idx > 0 {
                    let prev_w = tab_natural_widths[start_idx - 1] + if start_idx - 1 > 0 { 4 } else { 0 };
                    if used_w + prev_w <= total_w.saturating_sub(if end_idx < n_steps { 4 } else { 0 }) {
                        used_w += tab_natural_widths[start_idx - 1];
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
    let wrap_w = rows_area.width.saturating_sub(2).max(10) as usize;

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
                Paragraph::new(Line::from(Span::styled(
                    format!("+{} more", modal.commands.len() - max_cmds),
                    Style::default().fg(p.overlay0),
                ))),
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
            pairs.push(("⇥", "Tab".to_string()));
        }

        let current_row = modal.rows.get(modal.selected);
        if matches!(
            current_row,
            Some(ModalRow::Stepper { .. }) | Some(ModalRow::Choice { .. })
        ) {
            pairs.push(("←→", "Adjust".to_string()));
        }

        if modal.editing_text {
            pairs.push(("⏎", "Commit".to_string()));
        } else if matches!(current_row, Some(ModalRow::TextInput { .. })) {
            pairs.push(("⏎", "Edit".to_string()));
        } else if matches!(current_row, Some(ModalRow::Toggle { .. })) {
            pairs.push(("⏎", "Toggle".to_string()));
        } else if matches!(current_row, Some(ModalRow::Choice { .. })) {
            pairs.push(("⏎", "Select".to_string()));
        } else if !modal.commands.is_empty() && modal.selected >= modal.rows.len() {
            pairs.push(("⏎", "Run".to_string()));
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
                spans.push(Span::styled(" · ", Style::default().fg(p.overlay0)));
            }
            spans.push(Span::styled(
                key.to_string(),
                Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
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

fn role_color(p: &Palette, color: &str) -> Style {
    if color.is_empty() {
        Style::default().fg(p.blue).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(crate::ui::mod_bridge::color_from_name(p, color))
            .add_modifier(Modifier::BOLD)
    }
}

fn row_wrapped_lines(row: &ModalRow, w: usize) -> u16 {
    let w = w.max(10);
    let count = |text: &str| -> u16 {

        let visible: String = text
            .split('\x1b')
            .map(|seg| {
                let seg = seg.strip_prefix('[').unwrap_or(seg);
                let seg = seg.split_once('m').map(|(_, rest)| rest).unwrap_or(seg);
                seg.to_string()
            })
            .collect();
        let mut lines = 0u16;
        for logical in visible.split('\n') {
            let len = width(logical);
            lines += ((len as u16).div_ceil(w as u16)).max(1);
        }
        lines
    };
    match row {
        ModalRow::Info(t) => count(t),
        ModalRow::InfoColored { text, .. } => count(text),
        ModalRow::Separator(t) => count(t),
        ModalRow::Section { title, .. } => count(title),
        ModalRow::TextInput { label, value, .. } => count(&format!("{label}: {value}")),
        ModalRow::Table { headers, rows, .. } => {
            let mut n = count(&headers.join("  "));
            for r in rows {
                n += count(&r.join("  "));
            }
            n
        }
        _ => 1,
    }
}

fn row_spans(
    row: &ModalRow,
    p: &Palette,
    _selected: bool,
    w: usize,
    spinner_frame: &str,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    match row {
        ModalRow::Toggle { label, enabled, .. } => {
            let (glyph, fg) = if *enabled {
                ("✓", p.green)
            } else {
                ("✕", p.red)
            };
            let pill_w = 7;
            let label_max = w.saturating_sub(pill_w + 2);
            let label = truncate(label, label_max);
            let label_w = width(&label);
            spans.push(Span::styled(label, Style::default().fg(p.text)));
            let pad = w.saturating_sub(label_w + pill_w + 2);
            spans.push(Span::raw(" ".repeat(pad)));
            spans.push(Span::styled(" ‹ ", Style::default().fg(p.overlay0)));
            spans.push(Span::styled("[", Style::default().fg(p.overlay0)));
            spans.push(Span::styled(
                glyph,
                Style::default().fg(fg).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled("]", Style::default().fg(p.overlay0)));
            spans.push(Span::styled(" ›", Style::default().fg(p.overlay0)));
        }
        ModalRow::Stepper {
            label,
            value,
            unit,
            ..
        } => {
            let val_str = format!("{value}{unit}");
            let label_max = w.saturating_sub(width(&val_str) + 8);
            let label = truncate(label, label_max);
            let label_w = width(&label);
            spans.push(Span::styled(label, Style::default().fg(p.text)));
            let pad = w.saturating_sub(label_w + width(&val_str) + 8);
            spans.push(Span::raw(" ".repeat(pad)));
            spans.push(Span::styled(" ‹", Style::default().fg(p.accent).add_modifier(Modifier::BOLD)));
            spans.push(Span::styled(
                format!(" {val_str} "),
                Style::default().fg(p.text).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled("› ", Style::default().fg(p.accent).add_modifier(Modifier::BOLD)));
        }
        ModalRow::Choice {
            label,
            options,
            current,
            searchable,
            color,
            ..
        } => {

            let value = options
                .get(*current)
                .map(|(l, _, _)| l.as_str())
                .unwrap_or("");
            let is_free = value.to_lowercase().contains("free");
            let marker = if *searchable { " ▾" } else { "" };
            let value_style = if *searchable {
                Style::default().fg(p.accent).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(p.blue)
            };
            let label_style = if color.is_empty() {
                Style::default().fg(p.text)
            } else {
                Style::default()
                    .fg(crate::ui::mod_bridge::color_from_name(p, color))
                    .add_modifier(Modifier::BOLD)
            };
            let has_arrows = !*searchable && options.len() > 1;
            let val_display = if has_arrows {
                format!("‹ {value} ›")
            } else if *searchable && is_free {
                let clean = value.trim_end_matches(" · free").trim_end_matches(" free");
                format!("{clean} FREE{marker}")
            } else {
                format!("{value}{marker}")
            };
            let value_w = width(&val_display);
            let label_max = w.saturating_sub(value_w + 2);
            let label = truncate(label, label_max);
            let label_w = width(&label);
            spans.push(Span::styled(label, label_style));
            let pad = w.saturating_sub(label_w + value_w + 2);
            spans.push(Span::raw(" ".repeat(pad)));
            if has_arrows {
                spans.push(Span::styled("‹ ", Style::default().fg(p.overlay0)));
                spans.push(Span::styled(value.to_string(), value_style));
                spans.push(Span::styled(" ›", Style::default().fg(p.overlay0)));
            } else if *searchable && is_free {
                let clean = value.trim_end_matches(" · free").trim_end_matches(" free");
                spans.push(Span::styled(
                    clean.to_string(),
                    Style::default().fg(p.text).add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    "FREE",
                    Style::default()
                        .fg(p.green)
                        .add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled(
                    marker.to_string(),
                    Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::styled(val_display, value_style));
            }
        }
        ModalRow::TextInput { label, value, .. } => {
            let prefix = format!("{label}: ");

            let value_w = w.saturating_sub(width(&prefix) + 1);
            let value = truncate(value, value_w);
            spans.push(Span::styled(prefix, Style::default().fg(p.text)));
            spans.push(Span::styled(
                format!("{value}█"),
                Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
            ));
        }
        ModalRow::Info(text) => {

            if text.contains('\x1b') {
                spans.extend(ansi_spans(text, Style::default().fg(p.text)));
            } else if let Some((key, val)) = text.split_once(": ") {
                let key_span = Span::styled(format!("{}: ", key), Style::default().fg(p.subtext0));
                let val_style = match key {
                    k if k.contains("Cost") => {
                        Style::default().fg(p.yellow).add_modifier(Modifier::BOLD)
                    }
                    k if k.contains("Token") || k.contains("Model") => {
                        Style::default().fg(p.blue).add_modifier(Modifier::BOLD)
                    }
                    k if k.contains("Cache") => {
                        Style::default().fg(p.green).add_modifier(Modifier::BOLD)
                    }
                    k if k.contains("Turns") => {
                        Style::default().fg(p.mauve).add_modifier(Modifier::BOLD)
                    }
                    k if k.contains("YOLO") || k.contains("Auto-Approve") => {
                        if val.contains("ON") {
                            Style::default().fg(p.green).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(p.red).add_modifier(Modifier::BOLD)
                        }
                    }
                    _ => Style::default().fg(p.text),
                };
                spans.push(key_span);
                spans.push(Span::styled(val.to_string(), val_style));
            } else {
                spans.push(Span::styled(
                    text.clone(),
                    Style::default().fg(p.overlay1),
                ));
            }
        }
        ModalRow::InfoColored { text, color } => {
            let fg = color_from_name(&p, color);
            spans.push(Span::styled(
                text.clone(),
                Style::default().fg(fg).add_modifier(Modifier::BOLD),
            ));
        }
        ModalRow::Separator(title) => {
            let title_w = width(title);
            let dash_w = w.saturating_sub(title_w + 4);
            spans.push(Span::styled("── ", Style::default().fg(p.surface1)));
            spans.push(Span::styled(
                title.clone(),
                Style::default().fg(p.blue).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                format!(" {}", "─".repeat(dash_w)),
                Style::default().fg(p.surface1),
            ));
        }
        ModalRow::Progress {
            label,
            current,
            total,
        } => {
            let total = (*total).max(1);
            let pct = ((*current).min(total) * 100) / total;
            let done = *current >= total;

            let bar_w = 12usize.min(w.saturating_sub(12));
            let filled = (pct * bar_w) / 100;
            let bar: String = "█".repeat(filled) + &"░".repeat(bar_w.saturating_sub(filled));
            let spin = if done { "✓" } else { spinner_frame };
            let spin_style = if done {
                Style::default().fg(p.green)
            } else {
                Style::default().fg(p.accent).add_modifier(Modifier::BOLD)
            };
            let label_w = w.saturating_sub(bar_w + 8);
            let label = truncate(label, label_w);
            spans.push(Span::styled(" ", Style::default()));
            spans.push(Span::styled(spin.to_string(), spin_style));
            spans.push(Span::styled(
                format!(" {}  ", label),
                Style::default().fg(p.text),
            ));
            let bar_style = if done {
                Style::default().fg(p.green)
            } else {
                Style::default().fg(p.accent)
            };
            spans.push(Span::styled(bar, bar_style));
            spans.push(Span::styled(
                format!(" {}%", pct),
                Style::default().fg(p.yellow).add_modifier(Modifier::BOLD),
            ));
        }
        ModalRow::Table {
            headers,
            rows,
            color,
        } => {

            let ncols = headers.len().max(rows.iter().map(|r| r.len()).max().unwrap_or(0));
            if ncols == 0 {
                return spans;
            }
            let mut col_w: Vec<usize> = (0..ncols)
                .map(|c| {
                    headers
                        .get(c)
                        .map(|h| width(h))
                        .unwrap_or(0)
                        .max(
                            rows.iter()
                                .filter_map(|r| r.get(c).map(|cell| width(cell)))
                                .max()
                                .unwrap_or(0),
                        )
                })
                .collect();

            let total_w: usize = col_w.iter().sum::<usize>() + (ncols.saturating_sub(1) * 2);
            if total_w > w {
                let mut over = total_w - w;
                let mut order: Vec<usize> = (0..ncols).collect();
                order.sort_by(|a, b| col_w[*b].cmp(&col_w[*a]));
                for c in order {
                    if over == 0 {
                        break;
                    }
                    let cut = col_w[c].min(over);
                    col_w[c] -= cut;
                    over -= cut;
                }
            }
            let header_style = role_color(p, color);
            let header_line: Vec<Span> = (0..ncols)
                .flat_map(|c| {
                    let h = headers.get(c).map(|h| truncate(h, col_w[c])).unwrap_or_default();
                    let mut v = vec![Span::styled(h, header_style)];
                    if c + 1 < ncols {
                        v.push(Span::raw("  "));
                    }
                    v
                })
                .collect();
            spans.extend(header_line);
            spans.push(Span::raw(" "));
            for r in rows.iter().take(1) {
                spans.push(Span::raw("\n"));
                for c in 0..ncols {
                    let cell = r.get(c).map(|cell| truncate(cell, col_w[c])).unwrap_or_default();
                    let cell = format!(
                        "{}{}",
                        cell,
                        " ".repeat(col_w[c].saturating_sub(width(&cell)))
                    );
                    spans.push(Span::styled(cell, Style::default().fg(p.text)));
                    if c + 1 < ncols {
                        spans.push(Span::raw("  "));
                    }
                }
            }
        }
        ModalRow::Section { title, color } => {
            let title_w = width(title);
            let dash_w = w.saturating_sub(title_w + 4);
            spans.push(Span::styled("── ", Style::default().fg(p.surface1)));
            spans.push(Span::styled(title.clone(), role_color(p, color)));
            spans.push(Span::styled(
                format!(" {}", "─".repeat(dash_w)),
                Style::default().fg(p.surface1),
            ));
        }
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapped_lines_counts_overflow_and_multiline() {

        assert_eq!(row_wrapped_lines(&ModalRow::Info("hi".into()), 40), 1);

        assert_eq!(row_wrapped_lines(&ModalRow::Info("x".repeat(100)), 40), 3);

        assert_eq!(row_wrapped_lines(&ModalRow::Info("x".repeat(100)), 100), 1);

        assert_eq!(row_wrapped_lines(&ModalRow::Info("line one\nline two".into()), 40), 2);

        let ansi = "\u{1b}[32m● ON\u{1b}[0m";
        assert_eq!(row_wrapped_lines(&ModalRow::Info(ansi.into()), 40), 1);

        let long_ansi = format!("\u{1b}[32m{}\u{1b}[0m", "x".repeat(80));
        assert_eq!(row_wrapped_lines(&ModalRow::Info(long_ansi), 40), 2);

        let t = ModalRow::Table {
            headers: vec!["h1".into(), "h2".into()],
            rows: vec![vec!["a".into(), "b".into()]],
            color: String::new(),
        };
        assert_eq!(row_wrapped_lines(&t, 40), 2);
    }
}

