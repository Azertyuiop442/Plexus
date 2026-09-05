use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

use crate::theme::Palette;
use crate::ui::mod_bridge::color_from_name;
use crate::ui::text::{truncate, width};

use super::ansi::ansi_spans;
use super::model::ModalRow;

pub fn row_content_width(row: &ModalRow) -> u16 {
    let text = match row {
        ModalRow::Toggle { label, .. } => format!("{label}  ‹ [  ON ] ›"),
        ModalRow::Nav { label, .. } => format!("{label}  →"),
        ModalRow::Choice { label, options, .. } => {
            let cur = options
                .get(options.len().saturating_sub(1))
                .map(|(d, _, _)| d.clone())
                .unwrap_or_default();
            format!("{label}  ‹ {cur} ›")
        }
        ModalRow::TextInput { label, .. } => format!("{label}  ________"),
        ModalRow::Info(_) => String::new(),
        ModalRow::InfoColored { .. } => String::new(),
        ModalRow::Separator(t) => t.clone(),
        ModalRow::Progress { label, .. } => format!("{label}  0/0"),
        ModalRow::Stepper { label, value, unit, .. } => {
            let val_str = if *value == 0 && unit.contains("tries") {
                "Infinite (∞)".to_string()
            } else {
                format!("{value}{unit}")
            };
            format!("{label}  ‹ {val_str} ›")
        }
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
    (visible as u16).clamp(56, 72)
}

pub fn row_wrapped_lines(row: &ModalRow, w: usize) -> u16 {
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
            if len == 0 {
                lines = lines.saturating_add(1);
            } else {
                let wrapped = (len + w - 1) / w;
                lines = lines.saturating_add(wrapped as u16);
            }
        }
        lines.max(1)
    };

    match row {
        ModalRow::Toggle { .. } => 1,
        ModalRow::Choice { .. } => 1,
        ModalRow::Stepper { .. } => 1,
        ModalRow::TextInput { label, value, .. } => {
            let combined = format!("{label}: {value}");
            let lines = (width(&combined) + w - 1) / w;
            (lines as u16).max(1)
        }
        ModalRow::Info(text) => count(text),
        ModalRow::InfoColored { text, .. } => count(text),
        ModalRow::Separator(title) => {
            let title_w = width(title);
            let lines = (title_w + 4 + w - 1) / w;
            (lines as u16).max(1)
        }
        ModalRow::Progress { .. } => 1,
        ModalRow::Nav { .. } => 1,
        ModalRow::Table { rows, .. } => (rows.len().min(1) as u16).saturating_add(1),
        ModalRow::Section { title, .. } => {
            let title_w = width(title);
            let lines = (title_w + 4 + w - 1) / w;
            (lines as u16).max(1)
        }
    }
}

pub fn role_color(p: &Palette, color: &str) -> Style {
    if color.is_empty() {
        Style::default().fg(p.blue).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(crate::ui::mod_bridge::color_from_name(p, color))
            .add_modifier(Modifier::BOLD)
    }
}

pub fn row_spans(
    row: &ModalRow,
    p: &Palette,
    is_selected: bool,
    w: usize,
    spinner_frame: &str,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let cursor_prefix = if row.is_selectable() {
        if is_selected { "▎ " } else { "  " }
    } else {
        ""
    };
    let cursor_w = width(cursor_prefix);
    let right_pad = 1usize;
    let target_w = w.saturating_sub(cursor_w + right_pad);

    if !cursor_prefix.is_empty() {
        spans.push(Span::styled(
            cursor_prefix,
            if is_selected {
                Style::default().fg(p.accent).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(p.surface0)
            },
        ));
    }

    match row {
        ModalRow::Nav { label, color, .. } => {
            let fg = color_from_name(p, color);
            let ctrl_w = 2;
            let label_max = target_w.saturating_sub(ctrl_w + 1);
            let label = truncate(label, label_max);
            let label_w = width(&label);
            spans.push(Span::styled(
                label,
                Style::default().fg(fg).add_modifier(Modifier::BOLD),
            ));
            let pad = target_w.saturating_sub(label_w + ctrl_w);
            spans.push(Span::raw(" ".repeat(pad)));
            let nav_style = if is_selected {
                Style::default().fg(p.accent).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(p.overlay0)
            };
            spans.push(Span::styled(" →", nav_style));
            spans.push(Span::raw(" ".repeat(right_pad)));
        }
        ModalRow::Toggle { label, enabled, .. } => {
            let (state_text, fg) = if *enabled {
                ("  ON ", p.green)
            } else {
                (" OFF ", p.overlay0)
            };
            let ctrl_w = 11;
            let label_max = target_w.saturating_sub(ctrl_w + 1);
            let label = truncate(label, label_max);
            let label_w = width(&label);
            spans.push(Span::styled(label, Style::default().fg(p.text)));
            let pad = target_w.saturating_sub(label_w + ctrl_w);
            spans.push(Span::raw(" ".repeat(pad)));

            let bracket_style = if is_selected {
                Style::default().fg(p.accent).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(p.overlay0)
            };

            spans.push(Span::styled("‹ ", bracket_style));
            spans.push(Span::styled("[", Style::default().fg(p.surface1)));
            spans.push(Span::styled(
                state_text,
                Style::default().fg(fg).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled("]", Style::default().fg(p.surface1)));
            spans.push(Span::styled(" ›", bracket_style));
            spans.push(Span::raw(" ".repeat(right_pad)));
        }
        ModalRow::Stepper {
            label,
            value,
            unit,
            ..
        } => {
            let val_str = if *value == 0 && unit.contains("tries") {
                "Infinite (∞)".to_string()
            } else {
                format!("{value}{unit}")
            };
            let ctrl_w = width(&val_str) + 4;
            let label_max = target_w.saturating_sub(ctrl_w + 1);
            let label = truncate(label, label_max);
            let label_w = width(&label);
            spans.push(Span::styled(label, Style::default().fg(p.text)));
            let pad = target_w.saturating_sub(label_w + ctrl_w);
            spans.push(Span::raw(" ".repeat(pad)));

            let arrow_style = if is_selected {
                Style::default().fg(p.accent).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(p.overlay0)
            };

            spans.push(Span::styled("‹ ", arrow_style));
            spans.push(Span::styled(
                val_str,
                Style::default().fg(p.text).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(" ›", arrow_style));
            spans.push(Span::raw(" ".repeat(right_pad)));
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
                Style::default().fg(p.blue).add_modifier(Modifier::BOLD)
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
            let ctrl_w = width(&val_display);
            let label_max = target_w.saturating_sub(ctrl_w + 1);
            let label = truncate(label, label_max);
            let label_w = width(&label);
            spans.push(Span::styled(label, label_style));
            let pad = target_w.saturating_sub(label_w + ctrl_w);
            spans.push(Span::raw(" ".repeat(pad)));
            if has_arrows {
                let arrow_style = if is_selected {
                    Style::default().fg(p.accent).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(p.overlay0)
                };
                spans.push(Span::styled("‹ ", arrow_style));
                spans.push(Span::styled(value.to_string(), value_style));
                spans.push(Span::styled(" ›", arrow_style));
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
            spans.push(Span::raw(" ".repeat(right_pad)));
        }
        ModalRow::TextInput { label, value, .. } => {
            let prefix = if label.is_empty() {
                String::new()
            } else {
                format!("{label}: ")
            };
            let prefix_w = width(&prefix);
            let val_max = target_w.saturating_sub(prefix_w + 1);
            let val_str = truncate(value, val_max);
            spans.push(Span::styled(prefix, Style::default().fg(p.text)));
            spans.push(Span::styled(
                format!("{val_str}█"),
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
            let fg = color_from_name(p, color);
            spans.push(Span::styled(
                text.clone(),
                Style::default().fg(fg).add_modifier(Modifier::BOLD),
            ));
        }
        ModalRow::Separator(title) => {
            let title_w = width(title);
            let dash_w = target_w.saturating_sub(title_w + 4);
            spans.push(Span::styled("── ", Style::default().fg(p.surface1)));
            spans.push(Span::styled(
                title.clone(),
                Style::default().fg(p.blue).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                format!(" {}", "─".repeat(dash_w)),
                Style::default().fg(p.surface1),
            ));
            spans.push(Span::raw(" ".repeat(right_pad)));
        }
        ModalRow::Progress {
            label,
            current,
            total,
        } => {
            let total = (*total).max(1);
            let pct = ((*current).min(total) * 100) / total;
            let done = *current >= total;
            let bar_w = 12usize.min(target_w.saturating_sub(12));
            let filled = (pct * bar_w) / 100;
            let bar: String = "█".repeat(filled) + &"░".repeat(bar_w.saturating_sub(filled));
            let spin = if done { "✓" } else { spinner_frame };
            let spin_style = if done {
                Style::default().fg(p.green)
            } else {
                Style::default().fg(p.accent).add_modifier(Modifier::BOLD)
            };
            let pct_str = format!(" {}%", pct);
            let pct_w = width(&pct_str);
            let right_ctrl_w = bar_w + pct_w;
            let label_max = target_w.saturating_sub(right_ctrl_w + 5);
            let label = truncate(label, label_max);
            let label_w = width(&label);
            spans.push(Span::styled(" ", Style::default()));
            spans.push(Span::styled(spin.to_string(), spin_style));
            spans.push(Span::styled(
                format!(" {} ", label),
                Style::default().fg(p.text),
            ));
            let pad = target_w.saturating_sub(label_w + 4 + right_ctrl_w);
            spans.push(Span::raw(" ".repeat(pad)));
            let bar_style = if done {
                Style::default().fg(p.green)
            } else {
                Style::default().fg(p.accent)
            };
            spans.push(Span::styled(bar, bar_style));
            spans.push(Span::styled(
                pct_str,
                Style::default().fg(p.yellow).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(" ".repeat(right_pad)));
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
            if total_w > target_w {
                let mut over = total_w - target_w;
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
            let dash_w = target_w.saturating_sub(title_w + 4);
            spans.push(Span::styled("── ", Style::default().fg(p.surface1)));
            spans.push(Span::styled(title.clone(), role_color(p, color)));
            spans.push(Span::styled(
                format!(" {}", "─".repeat(dash_w)),
                Style::default().fg(p.surface1),
            ));
            spans.push(Span::raw(" ".repeat(right_pad)));
        }
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_on_and_off_have_identical_character_width() {
        let p = Palette::dark();
        let row_on = ModalRow::Toggle {
            key: "toggle_test".into(),
            label: "Enable Feature".into(),
            enabled: true,
        };
        let row_off = ModalRow::Toggle {
            key: "toggle_test".into(),
            label: "Enable Feature".into(),
            enabled: false,
        };

        let w = 64usize;
        for is_sel in [false, true] {
            let spans_on = row_spans(&row_on, &p, is_sel, w, "|");
            let spans_off = row_spans(&row_off, &p, is_sel, w, "|");

            let text_on: String = spans_on.iter().map(|s| s.content.as_ref()).collect();
            let text_off: String = spans_off.iter().map(|s| s.content.as_ref()).collect();

            assert_eq!(text_on.chars().count(), text_off.chars().count());
            assert_eq!(text_on.chars().count(), w);
            assert_eq!(text_off.chars().count(), w);

            let end_bracket_on = text_on.rfind(']').unwrap();
            let end_bracket_off = text_off.rfind(']').unwrap();
            assert_eq!(end_bracket_on, end_bracket_off);

            let end_chevron_on = text_on.rfind('›').unwrap();
            let end_chevron_off = text_off.rfind('›').unwrap();
            assert_eq!(end_chevron_on, end_chevron_off);
        }
    }

    #[test]
    fn all_modal_controls_align_to_exact_same_column() {
        let p = Palette::dark();
        let w = 70usize;

        let row_toggle_on = ModalRow::Toggle {
            key: "t_on".into(),
            label: "Toggle Alpha".into(),
            enabled: true,
        };
        let row_toggle_off = ModalRow::Toggle {
            key: "t_off".into(),
            label: "Toggle Beta Long Label".into(),
            enabled: false,
        };
        let row_stepper = ModalRow::Stepper {
            key: "step".into(),
            label: "Step Counter".into(),
            value: 10,
            min: 0,
            max: 100,
            step: 5,
            unit: " ms".into(),
        };
        let row_choice_arrows = ModalRow::Choice {
            key: "choice1".into(),
            label: "Mode Choice".into(),
            options: vec![
                ("Option A".into(), "a".into(), String::new()),
                ("Option B".into(), "b".into(), String::new()),
            ],
            current: 0,
            searchable: false,
            color: String::new(),
        };
        let row_choice_search = ModalRow::Choice {
            key: "choice2".into(),
            label: "Model Picker".into(),
            options: vec![("claude-3-7-sonnet".into(), "c37".into(), String::new())],
            current: 0,
            searchable: true,
            color: String::new(),
        };

        let check_end_pos = |row: &ModalRow| -> usize {
            let spans = row_spans(row, &p, false, w, "|");
            let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
            assert_eq!(text.chars().count(), w);
            text.trim_end().chars().count()
        };

        assert_eq!(check_end_pos(&row_toggle_on), w - 1);
        assert_eq!(check_end_pos(&row_toggle_off), w - 1);
        assert_eq!(check_end_pos(&row_stepper), w - 1);
        assert_eq!(check_end_pos(&row_choice_arrows), w - 1);
        assert_eq!(check_end_pos(&row_choice_search), w - 1);
    }
}
