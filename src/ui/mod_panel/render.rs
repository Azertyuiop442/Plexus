
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::Frame;

use crate::theme::Palette;
use crate::ui::mod_bridge::{color_from_name, contract::ModPanel};
use crate::ui::text::truncate;
use crate::ui::widget::SelectableRow;

use super::state::{PanelState, PanelView};

fn fill_area(frame: &mut ratatui::Frame, area: Rect, bg: Color) {
    let width = area.width as usize;
    let spaces = " ".repeat(width);
    let style = Style::default().bg(bg);
    let buf = frame.buffer_mut();
    for y in area.top()..area.bottom() {
        buf.set_string(area.left(), y, &spaces, style);
    }
}

fn row_area(inner: Rect, y: u16) -> Rect {
    Rect::new(inner.left(), y, inner.width, 1)
}

fn card_row(
    frame: &mut ratatui::Frame,
    y: &mut u16,
    inner: Rect,
    mut spans: Vec<Span>,
    selected: bool,
    focused: bool,
    p: &Palette,
) {
    if *y >= inner.bottom() {
        return;
    }
    let bar_color = if selected && focused {
        Palette::dark().blue
    } else {
        crate::theme::effective_bg()
    };
    let mut new_spans = vec![Span::styled(" ", Style::default().bg(bar_color))];
    if selected && focused {
        new_spans[0] = Span::styled("▎", Style::default().fg(bar_color));
    }
    new_spans.append(&mut spans);
    frame.render_widget(SelectableRow::new(new_spans, selected, &p), row_area(inner, *y));
    *y += 1;
}

pub fn render_panel(
    frame: &mut Frame,
    area: Rect,
    panel: &ModPanel,
    st: &PanelState,
    focused: bool,
    view: &mut PanelView,
) {
    let p = Palette::dark();
    if area.width < 12 || area.height < 6 {
        return;
    }

    view.row_y.clear();
    view.tab_y.clear();
    view.action_x.clear();
    view.carousel_arrows.clear();

    let bg = crate::theme::effective_bg();
    fill_area(frame, area, bg);

    let border_color = if focused {
        Palette::dark().blue
    } else {
        p.surface1
    };
    let border_style = Style::default().fg(border_color);
    let (x0, y0, box_w, box_h) = (area.x, area.y, area.width, area.height);
    for x in x0 + 1..x0 + box_w.saturating_sub(1) {
        frame.buffer_mut()[(x, y0)].set_symbol("─").set_style(border_style);
        frame.buffer_mut()[(x, y0 + box_h - 1)].set_symbol("─").set_style(border_style);
    }
    let mid_y = y0 + box_h / 2;
    for y in y0 + 1..y0 + box_h.saturating_sub(1) {
        let is_grip = y >= mid_y.saturating_sub(1) && y <= mid_y + 1;
        let (sym, style) = if is_grip {
            ("║", Style::default().fg(p.accent).add_modifier(Modifier::BOLD))
        } else {
            ("│", border_style)
        };
        frame.buffer_mut()[(x0, y)].set_symbol(sym).set_style(style);
        frame.buffer_mut()[(x0 + box_w - 1, y)].set_symbol("│").set_style(border_style);
    }
    frame.buffer_mut()[(x0, y0)].set_symbol("╭").set_style(border_style);
    frame.buffer_mut()[(x0 + box_w - 1, y0)].set_symbol("╮").set_style(border_style);
    frame.buffer_mut()[(x0, y0 + box_h - 1)].set_symbol("╰").set_style(border_style);
    frame.buffer_mut()[(x0 + box_w - 1, y0 + box_h - 1)].set_symbol("╯").set_style(border_style);

    let resolved_icon = {
        let icon_str = if panel.icon.is_empty() { "nf-dev-git" } else { &panel.icon };
        crate::ui::glyph::resolve_glyph(icon_str)
    };
    let icon_prefix = if !resolved_icon.is_empty() {
        format!("{} ", resolved_icon)
    } else {
        String::new()
    };
    let raw_title = if panel.title == "git · status" || panel.title.is_empty() {
        "git"
    } else {
        &panel.title
    };
    let max_hdr_title_w = (box_w.saturating_sub(6) as usize).saturating_sub(crate::ui::text::width(&icon_prefix));
    let trunc_title = truncate(raw_title, max_hdr_title_w);
    let header = format!(" {}{} ", icon_prefix, trunc_title);
    let header_style = Style::default()
        .fg(if focused { Palette::dark().blue } else { p.blue })
        .add_modifier(Modifier::BOLD);
    for (i, ch) in header.chars().enumerate() {
        let cx = x0 + 2 + i as u16;
        if cx < x0 + box_w - 2 {
            frame.buffer_mut()[(cx, y0)].set_symbol(&ch.to_string()).set_style(header_style);
        }
    }

    let inner = Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), area.height.saturating_sub(2));
    let width = inner.width as usize;
    let mut y = inner.top();

    if !panel.tabs.is_empty() {
        let tabs_y = y;
        let mut tab_items: Vec<(usize, String, u16, bool)> = Vec::new();
        let mut total_tabs_w: u16 = 0;
        for (ti, tab) in panel.tabs.iter().enumerate() {
            let is_active = ti == st.active_tab;
            let raw_tab_label = if tab.label == "Status" || tab.label == "status" {
                "Files"
            } else {
                &tab.label
            };
            let icon_str = if tab.icon.is_empty() && (tab.id == "status" || tab.label == "Status" || tab.label == "Files") {
                "nf-oct-checklist"
            } else {
                &tab.icon
            };
            let resolved_icon = crate::ui::glyph::resolve_glyph(icon_str);
            let item = if !resolved_icon.is_empty() {
                format!(" {} {} ", resolved_icon, raw_tab_label)
            } else {
                format!(" {} ", raw_tab_label)
            };
            let w = crate::ui::text::width(&item) as u16;
            total_tabs_w += w + 1;
            tab_items.push((ti, item, w, is_active));
        }

        if total_tabs_w <= inner.width {

            let start_x = inner.left() + (inner.width.saturating_sub(total_tabs_w)) / 2;
            let mut tx = start_x;
            for (ti, item, w, is_active) in tab_items {
                if tx + w > inner.right() {
                    break;
                }
                let style = if is_active {
                    Style::default().fg(p.panel_bg).bg(p.accent).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(p.overlay1).bg(p.surface0)
                };
                let span = Span::styled(&item, style);
                frame.buffer_mut().set_line(tx, tabs_y, &Line::from(vec![span]), w);
                view.tab_x.push((tx, tx + w, tabs_y, ti));
                tx += w + 1;
            }
        } else {

            let tab = &panel.tabs[st.active_tab.min(panel.tabs.len() - 1)];
            let raw_tab_label = if tab.label == "Status" || tab.label == "status" { "Files" } else { &tab.label };
            let icon_str = if tab.icon.is_empty() && (tab.id == "status" || tab.label == "Status" || tab.label == "Files") {
                "nf-oct-checklist"
            } else {
                &tab.icon
            };
            let resolved_icon = crate::ui::glyph::resolve_glyph(icon_str);
            let tab_icon = if !resolved_icon.is_empty() { format!("{} ", resolved_icon) } else { String::new() };
            let max_tab_label_w = width.saturating_sub(crate::ui::text::width(&tab_icon) + 8).max(2);
            let trunc_tab_label = truncate(raw_tab_label, max_tab_label_w);
            let label = format!(" {}{} ", tab_icon, trunc_tab_label);
            let label_len = crate::ui::text::width(&label);
            let side = (width.saturating_sub(label_len + 6)) / 2;
            let arrow_style = Style::default().fg(p.overlay1).add_modifier(Modifier::BOLD);
            let mut spans = vec![
                Span::styled(" ", Style::default().fg(p.text)),
                Span::styled("←", arrow_style),
                Span::styled("  ", Style::default().fg(p.text)),
                Span::raw(" ".repeat(side)),
            ];
            spans.push(Span::styled(
                label,
                Style::default().fg(p.text).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(" ".repeat(width.saturating_sub(label_len + 6 + side))));
            spans.push(Span::styled("→", arrow_style));
            let line = Line::from(spans);
            frame.buffer_mut().set_line(inner.left(), tabs_y, &line, inner.width);
            view.carousel_arrows.push((inner.left() + 1, inner.left() + 4, tabs_y, false));
            let right_arrow_x = inner.right().saturating_sub(3);
            view.carousel_arrows.push((right_arrow_x, inner.right(), tabs_y, true));
            view.tab_x.push((inner.left() + 4, right_arrow_x, tabs_y, (st.active_tab + 1) % panel.tabs.len()));
        }
        y += 1;

        if y < inner.bottom() {
            frame.buffer_mut().set_line(
                inner.left() + 1,
                y,
                &Line::from(vec![Span::styled(
                    "─".repeat(width.saturating_sub(2)),
                    Style::default().fg(p.surface1),
                )]),
                inner.width.saturating_sub(1),
            );
            y += 1;
        }
    }

    if (st.active_tab == 0 || panel.tabs.is_empty()) && !panel.footer.is_empty() {
        let action_y = y;
        let mut action_items: Vec<(usize, String, u16)> = Vec::new();
        let mut total_actions_w: u16 = 0;
        for (fi, hint) in panel.footer.iter().enumerate() {
            if hint.is_action || !hint.icon.is_empty() {
                let resolved_action_icon = crate::ui::glyph::resolve_glyph(&hint.icon);
                let item = if !resolved_action_icon.is_empty() {
                    format!(" {} {} ", resolved_action_icon, hint.label)
                } else {
                    format!(" {} ", hint.label)
                };
                let w = crate::ui::text::width(&item) as u16;
                total_actions_w += w;
                action_items.push((fi, item, w));
            }
        }

        if total_actions_w > inner.width {
            action_items.clear();
            total_actions_w = 0;
            for (fi, hint) in panel.footer.iter().enumerate() {
                if hint.is_action || !hint.icon.is_empty() {
                    let resolved_action_icon = crate::ui::glyph::resolve_glyph(&hint.icon);
                    let item = if !resolved_action_icon.is_empty() {
                        format!(" {} ", resolved_action_icon)
                    } else {
                        format!(" {} ", hint.label.chars().next().unwrap_or('?'))
                    };
                    let w = crate::ui::text::width(&item) as u16;
                    total_actions_w += w;
                    action_items.push((fi, item, w));
                }
            }
        }

        if !action_items.is_empty() {
            let start_x = inner.left() + (inner.width.saturating_sub(total_actions_w)) / 2;
            let mut ax = start_x;
            for (fi, item, w) in action_items {
                if ax + w > inner.right() {
                    break;
                }
                let style = if fi == st.active_action {
                    Style::default().fg(p.accent).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(p.subtext0)
                };
                let span = Span::styled(&item, style);
                frame.buffer_mut().set_line(ax, action_y, &Line::from(vec![span]), w);
                view.action_x.push((ax, ax + w, action_y, fi));
                ax += w;
            }
            y += 1;

            if y < inner.bottom() {
                frame.buffer_mut().set_line(
                    inner.left() + 1,
                    y,
                    &Line::from(vec![Span::styled(
                        "─".repeat(width.saturating_sub(2)),
                        Style::default().fg(p.surface1),
                    )]),
                    inner.width.saturating_sub(1),
                );
                y += 1;
            }
        }
    }

    match panel.state.as_str() {
        "loading" => {
            let line = Line::from(vec![Span::styled(
                " loading…",
                Style::default().fg(p.yellow).add_modifier(Modifier::BOLD),
            )]);
            frame.buffer_mut().set_line(inner.left() + 1, y, &line, inner.width.saturating_sub(1));
            return;
        }
        "error" => {
            let line = Line::from(vec![Span::styled(
                format!(" ⚠ {}", panel.error),
                Style::default().fg(p.red).add_modifier(Modifier::BOLD),
            )]);
            frame.buffer_mut().set_line(inner.left() + 1, y, &line, inner.width.saturating_sub(1));
            return;
        }
        "empty" | _ => {}
    }

    let tab_id = panel.tabs.get(st.active_tab).map(|t| t.id.as_str()).unwrap_or("status");
    let active_rows = panel
        .tab_rows
        .get(tab_id)
        .or_else(|| if st.active_tab == 0 || tab_id == "status" { Some(&panel.rows) } else { None })
        .unwrap_or(&panel.rows);

    if active_rows.is_empty() {
        let line = Line::from(vec![Span::styled(
            " (empty)",
            Style::default().fg(p.overlay0),
        )]);
        frame.buffer_mut().set_line(inner.left() + 1, y, &line, inner.width.saturating_sub(1));
        return;
    }

    let visible = inner.height.saturating_sub(1) as usize;
    let mut st = st.clone();
    st.set_visible(visible, active_rows.len());

    for (i, row) in active_rows.iter().enumerate().skip(st.scroll).take(visible) {
        if y >= inner.bottom() {
            break;
        }
        let selected = i == st.selected;

        if !row.spans.is_empty() {
            let mut badge_spans: Vec<Span> = Vec::new();
            let mut left_main_span: Option<(&str, Style)> = None;
            let mut right_spans: Vec<Span> = Vec::new();
            let mut badge_w: usize = 0;
            let mut right_w: usize = 0;

            for span in &row.spans {
                let color = if selected && span.align != "badge" {
                    p.text
                } else if !span.color.is_empty() {
                    color_from_name(&p, &span.color)
                } else {
                    p.text
                };
                let style = Style::default()
                    .fg(color)
                    .add_modifier(if span.bold || selected { Modifier::BOLD } else { Modifier::empty() });

                if span.align == "right" {
                    let text_w = crate::ui::text::width(&span.text);
                    right_w += text_w;
                    right_spans.push(Span::styled(span.text.clone(), style));
                } else if span.align == "badge" {
                    badge_w += crate::ui::text::width(&span.text);
                    badge_spans.push(Span::styled(span.text.clone(), style));
                } else {

                    left_main_span = Some((&span.text, style));
                }
            }

            right_spans.push(Span::styled(
                " ›",
                if selected {
                    Style::default().fg(p.text).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(p.overlay0)
                },
            ));
            right_w += 2;

            let avail_w = (width.saturating_sub(1)).saturating_sub(badge_w + right_w);
            let mut spans = badge_spans;
            let mut used_w = 1 + badge_w + right_w;

            if let Some((main_text, style)) = left_main_span {
                if avail_w > 0 {
                    let trunc = truncate(main_text, avail_w);
                    let tw = crate::ui::text::width(&trunc);
                    used_w += tw;
                    spans.push(Span::styled(trunc, style));
                }
            }

            let gap = width.saturating_sub(used_w);
            if gap > 0 {
                spans.push(Span::raw(" ".repeat(gap)));
            }
            spans.extend(right_spans);

            view.row_y.push((y, i));
            card_row(frame, &mut y, inner, spans, selected, focused, &p);
            continue;
        }

        let status = row.cells.first().cloned().unwrap_or_default();
        let status_color = if !row.color.is_empty() {
            color_from_name(&p, &row.color)
        } else {
            p.accent
        };
        let mut spans = vec![Span::styled(
            format!(" {} ", status),
            Style::default().fg(if selected { p.text } else { status_color }).add_modifier(Modifier::BOLD),
        )];
        let mut left_len = crate::ui::text::width(&format!(" {} ", status));
        for cell in row.cells.iter().skip(1) {
            let cell = truncate(cell, width.saturating_sub(left_len + 4).max(4));
            spans.push(Span::styled(cell.clone(), Style::default().fg(if selected { p.text } else { p.subtext0 })));
            left_len += crate::ui::text::width(&cell) + 1;
        }
        let gap = width.saturating_sub(left_len + 2);
        if gap > 0 {
            spans.push(Span::raw(" ".repeat(gap)));
        }
        spans.push(Span::styled(
            "›",
            if selected { Style::default().fg(p.text).add_modifier(Modifier::BOLD) } else { Style::default().fg(p.overlay0) },
        ));
        view.row_y.push((y, i));
        card_row(frame, &mut y, inner, spans, selected, focused, &p);
    }

    if let Some(summary) = &panel.summary {

        if y < inner.bottom() {
            frame.buffer_mut().set_line(
                inner.left() + 1,
                y,
                &Line::from(vec![Span::styled(
                    "─".repeat(width.saturating_sub(2)),
                    Style::default().fg(p.surface1),
                )]),
                inner.width.saturating_sub(1),
            );
            y += 1;
        }

        if y < inner.bottom() {
            let label = &summary.label;
            if label.contains(" · ") && width < 28 {
                let parts: Vec<&str> = label.splitn(2, " · ").collect();
                let branch_part = parts[0];
                let detail_part = parts.get(1).copied().unwrap_or("");

                let spans1 = vec![
                    Span::styled("◈ ", Style::default().fg(p.green)),
                    Span::styled(
                        branch_part.to_string(),
                        Style::default()
                            .fg(color_from_name(&p, &summary.color))
                            .add_modifier(Modifier::UNDERLINED),
                    ),
                ];
                card_row(frame, &mut y, inner, spans1, false, focused, &p);

                if !detail_part.is_empty() && y < inner.bottom() {
                    let max_d_w = width.saturating_sub(4);
                    let trunc_detail = truncate(detail_part, max_d_w);
                    let spans2 = vec![
                        Span::raw("  "),
                        Span::styled(
                            trunc_detail,
                            Style::default().fg(p.subtext0),
                        ),
                    ];
                    card_row(frame, &mut y, inner, spans2, false, focused, &p);
                }
            } else {
                let max_sum_w = width.saturating_sub(4);
                let trunc_sum = truncate(label, max_sum_w);
                let label_len = crate::ui::text::width(&trunc_sum);
                let spans = vec![
                    Span::styled("◈ ", Style::default().fg(p.green)),
                    Span::styled(
                        trunc_sum,
                        Style::default()
                            .fg(color_from_name(&p, &summary.color))
                            .add_modifier(Modifier::UNDERLINED),
                    ),
                    Span::styled(
                        format!(" ─{}", "─".repeat(width.saturating_sub(label_len + 4))),
                        Style::default().fg(p.surface1),
                    ),
                ];
                card_row(frame, &mut y, inner, spans, false, focused, &p);
            }
        }
    }
}

