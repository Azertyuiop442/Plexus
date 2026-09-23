use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::theme::Palette;
use crate::ui::widget::SelectableRow;

use super::models::{SettingsSubMenu, SidebarRow, SidebarView};
use super::state::Sidebar;

fn label_style(p: &Palette, selected: bool) -> Style {
    if selected {
        Style::default().fg(p.text).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(p.subtext0)
    }
}

fn toggle_span<'a>(p: &Palette, on: bool, _selected: bool) -> Span<'a> {
    if on {
        Span::styled(
            "[✓]",
            Style::default().fg(p.green).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            "[✕]",
            Style::default().fg(p.red).add_modifier(Modifier::BOLD),
        )
    }
}

fn pad(width: usize, left: usize, right: usize) -> String {
    " ".repeat(width.saturating_sub(left + right + 1))
}

fn card_row(
    frame: &mut ratatui::Frame,
    y: &mut u16,
    inner: Rect,
    view: &mut SidebarView,
    row_type: SidebarRow,
    mut spans: Vec<Span>,
    selected: bool,
    focused: bool,
    p: &Palette,
) {
    if *y < inner.bottom() {
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
        spans = new_spans;
        view.row_y.push((*y, row_type));
        let row_area = Rect::new(inner.left(), *y, inner.width, 1);
        frame.render_widget(SelectableRow::new(spans, selected, p), row_area);
        *y += 1;
    }
}

pub fn render_settings_menu(
    frame: &mut ratatui::Frame,
    inner: Rect,
    width: usize,
    y: &mut u16,
    sidebar: &Sidebar,
    selected_row: Option<SidebarRow>,
    focused: bool,
    view: &mut SidebarView,
    p: &Palette,
) {
    match sidebar.settings_menu {
        SettingsSubMenu::Main => {
            for (row_type, icon, full_label, short_label) in [
                (SidebarRow::NavPreferences, nf_icons::nf!("nf-cod-settings_gear"), "Preferences", "Prefs"),
                (SidebarRow::NavModConfig, nf_icons::nf!("nf-cod-extensions"), "Mod Config", "Mods"),
                (SidebarRow::NavAIPrefs, nf_icons::nf!("nf-cod-sparkle"), "AI Prefs", "AI"),
            ] {
                let sel = selected_row == Some(row_type);
                let chevron = Span::styled(
                    "›",
                    if sel {
                        Style::default().fg(p.text).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(p.overlay0)
                    },
                );
                let spans = if width <= 3 {
                    vec![Span::styled(format!("{icon}"), label_style(p, sel))]
                } else if width < 6 {
                    vec![
                        Span::styled(format!("{icon} "), label_style(p, sel)),
                        chevron,
                    ]
                } else if width < 16 {
                    let avail_text = width.saturating_sub(5);
                    let short: String = short_label.chars().take(avail_text).collect();
                    let left = format!(" {icon} {short}");
                    let left_len = crate::ui::text::width(&left);
                    let pad_count = width.saturating_sub(left_len + 1 + 1);
                    vec![
                        Span::styled(left, label_style(p, sel)),
                        Span::raw(" ".repeat(pad_count)),
                        chevron,
                    ]
                } else {
                    let left = format!(" {} {} ", icon, full_label);
                    let left_len = crate::ui::text::width(&left);
                    vec![
                        Span::styled(left, label_style(p, sel)),
                        Span::raw(pad(width, left_len, 1)),
                        chevron,
                    ]
                };
                card_row(frame, y, inner, view, row_type, spans, sel, focused, p);
            }
        }
        SettingsSubMenu::Preferences => {
            let sel = selected_row == Some(SidebarRow::NavBack);
            let back_icon = nf_icons::nf!("nf-cod-arrow_left");
            let chevron = Span::styled(
                "›",
                if sel {
                    Style::default().fg(p.text).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(p.overlay0)
                },
            );
            let spans = if width <= 3 {
                vec![Span::styled(
                    format!("{back_icon}"),
                    if sel { label_style(p, true) } else { Style::default().fg(p.accent) },
                )]
            } else if width < 6 {
                vec![
                    Span::styled(
                        format!("{back_icon} "),
                        if sel { label_style(p, true) } else { Style::default().fg(p.accent) },
                    ),
                    chevron,
                ]
            } else if width < 16 {
                let avail_text = width.saturating_sub(5);
                let short: String = "Back".chars().take(avail_text).collect();
                let left = format!(" {back_icon} {short}");
                let left_len = crate::ui::text::width(&left);
                let pad_count = width.saturating_sub(left_len + 1 + 1);
                vec![
                    Span::styled(
                        left,
                        if sel { label_style(p, true) } else { Style::default().fg(p.accent) },
                    ),
                    Span::raw(" ".repeat(pad_count)),
                    chevron,
                ]
            } else {
                let left = format!(" {back_icon} Back");
                let left_w = crate::ui::text::width(&left);
                let pad_count = width.saturating_sub(left_w + 1 + 1).max(2);
                vec![
                    Span::styled(
                        left,
                        if sel {
                            label_style(p, true)
                        } else {
                            Style::default().fg(p.accent)
                        },
                    ),
                    Span::raw(" ".repeat(pad_count)),
                    chevron,
                ]
            };
            card_row(frame, y, inner, view, SidebarRow::NavBack, spans, sel, focused, p);

            let sel = selected_row == Some(SidebarRow::PrefFullConfig);
            let edit_icon = nf_icons::nf!("nf-cod-edit");
            let chevron_cfg = Span::styled(
                "›",
                if sel {
                    Style::default().fg(p.text).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(p.yellow)
                },
            );
            let spans = if width <= 3 {
                vec![Span::styled(format!("{edit_icon}"), label_style(p, sel))]
            } else if width < 6 {
                vec![
                    Span::styled(format!("{edit_icon} "), label_style(p, sel)),
                    chevron_cfg,
                ]
            } else if width < 16 {
                let avail_text = width.saturating_sub(5);
                let short: String = "Config".chars().take(avail_text).collect();
                let left = format!(" {edit_icon} {short}");
                let left_len = crate::ui::text::width(&left);
                let pad_count = width.saturating_sub(left_len + 1 + 1);
                vec![
                    Span::styled(left, label_style(p, sel)),
                    Span::raw(" ".repeat(pad_count)),
                    chevron_cfg,
                ]
            } else {
                let left = " Full Config".to_string();
                let right = "edit ›";
                let left_w = crate::ui::text::width(&left);
                let right_w = crate::ui::text::width(right);
                let pad_count = width.saturating_sub(left_w + right_w + 1).max(2);
                vec![
                    Span::styled(left, label_style(p, sel)),
                    Span::raw(" ".repeat(pad_count)),
                    Span::styled(
                        right,
                        if sel {
                            Style::default().fg(p.text).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(p.yellow)
                        },
                    ),
                ]
            };
            card_row(frame, y, inner, view, SidebarRow::PrefFullConfig, spans, sel, focused, p);

            let sel = selected_row == Some(SidebarRow::PrefAutoRetry);
            let spans = if width < 6 {
                let sym = if sidebar.auto_retry_enabled { "[✓]" } else { "[✕]" };
                let col = if sidebar.auto_retry_enabled { p.green } else { p.red };
                vec![Span::styled(format!("{sym}"), Style::default().fg(col).add_modifier(Modifier::BOLD))]
            } else if width < 16 {
                let right = if sidebar.auto_retry_enabled { "ON" } else { "OFF" };
                let right_style = if sidebar.auto_retry_enabled {
                    Style::default().fg(p.green).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(p.overlay0)
                };
                let avail_label = width.saturating_sub(5);
                let short: String = "Retry".chars().take(avail_label).collect();
                let left = format!(" {short}");
                let left_w = crate::ui::text::width(&left);
                let right_w = right.len();
                let pad_count = width.saturating_sub(left_w + right_w + 1).max(1);
                vec![
                    Span::styled(left, label_style(p, sel)),
                    Span::raw(" ".repeat(pad_count)),
                    Span::styled(right, right_style),
                ]
            } else {
                let left = " Error Recovery".to_string();
                let right = if sidebar.auto_retry_enabled { "ON ›" } else { "OFF ›" };
                let left_w = crate::ui::text::width(&left);
                let right_w = crate::ui::text::width(right);
                let pad_count = width.saturating_sub(left_w + right_w + 1).max(2);
                vec![
                    Span::styled(left, label_style(p, sel)),
                    Span::raw(" ".repeat(pad_count)),
                    Span::styled(
                        right,
                        if sel {
                            Style::default().fg(p.text).add_modifier(Modifier::BOLD)
                        } else if sidebar.auto_retry_enabled {
                            Style::default().fg(p.green)
                        } else {
                            Style::default().fg(p.overlay0)
                        },
                    ),
                ]
            };
            card_row(frame, y, inner, view, SidebarRow::PrefAutoRetry, spans, sel, focused, p);

            let sel = selected_row == Some(SidebarRow::PrefSkills);
            let tools_icon = nf_icons::nf!("nf-cod-tools");
            let chevron_skills = Span::styled(
                "›",
                if sel {
                    Style::default().fg(p.text).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(p.overlay0)
                },
            );
            let spans = if width <= 3 {
                vec![Span::styled(format!("{tools_icon}"), label_style(p, sel))]
            } else if width < 6 {
                vec![
                    Span::styled(format!("{tools_icon} "), label_style(p, sel)),
                    chevron_skills,
                ]
            } else if width < 16 {
                let avail_text = width.saturating_sub(5);
                let short: String = "Skills".chars().take(avail_text).collect();
                let left = format!(" {tools_icon} {short}");
                let left_len = crate::ui::text::width(&left);
                let pad_count = width.saturating_sub(left_len + 1 + 1);
                vec![
                    Span::styled(left, label_style(p, sel)),
                    Span::raw(" ".repeat(pad_count)),
                    chevron_skills,
                ]
            } else {
                let left = " Skills".to_string();
                let right = if sidebar.skills_update_count > 0 {
                    let n = sidebar.skills_update_count;
                    let noun = if n == 1 { "update" } else { "updates" };
                    format!("+{n} {noun} ›")
                } else {
                    "manage ›".to_string()
                };
                let left_w = crate::ui::text::width(&left);
                let right_w = crate::ui::text::width(&right);
                let pad_count = width.saturating_sub(left_w + right_w + 1).max(2);
                let count = sidebar.skills_update_count;
                let right_color = if count > 0 { p.yellow } else { p.overlay0 };
                vec![
                    Span::styled(left, label_style(p, sel)),
                    Span::raw(" ".repeat(pad_count)),
                    Span::styled(
                        right,
                        if sel {
                            Style::default().fg(p.text).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(right_color)
                        },
                    ),
                ]
            };
            card_row(frame, y, inner, view, SidebarRow::PrefSkills, spans, sel, focused, p);

            *y += 1;
            if *y < inner.bottom() {
                frame.buffer_mut().set_line(
                    inner.left() + 1,
                    *y,
                    &Line::from(vec![Span::styled(
                        "─".repeat(width.saturating_sub(2)),
                        Style::default().fg(p.surface1),
                    )]),
                    inner.width.saturating_sub(1),
                );
                *y += 1;
            }

            for (row_type, label, abbrev, on) in [
                (SidebarRow::PrefSkillInjection, "Skill Injection", "Skills", Some(sidebar.skill_injection)),
                (SidebarRow::PrefYolo, "YOLO Mode", "YOLO", Some(sidebar.yolo_mode)),
                (SidebarRow::PrefShowUsage, "Show Usage", "Usage", Some(sidebar.show_usage)),
                (SidebarRow::PrefSounds, "Sound Alerts", "Sounds", Some(sidebar.sound_notifications)),
                (SidebarRow::PrefWebhook, "Status Webhook", "Webhook", Some(sidebar.webhook_enabled)),
            ] {
                let sel = selected_row == Some(row_type);
                let value_span = if let Some(on) = on {
                    toggle_span(p, on, sel)
                } else {
                    Span::raw("")
                };
                let spans = if width < 6 {
                    vec![value_span]
                } else if width < 16 {
                    let avail_label = width.saturating_sub(5);
                    let short: String = abbrev.chars().take(avail_label).collect();
                    let left = format!(" {short}");
                    let left_w = crate::ui::text::width(&left);
                    let right_w = 3;
                    let pad_count = width.saturating_sub(left_w + right_w + 1).max(1);
                    vec![
                        Span::styled(left, label_style(p, sel)),
                        Span::raw(" ".repeat(pad_count)),
                        value_span,
                    ]
                } else {
                    let left = format!(" {label}");
                    let left_w = crate::ui::text::width(&left);
                    let right_w = 3;
                    let pad_count = width.saturating_sub(left_w + right_w + 1).max(2);
                    vec![
                        Span::styled(left, label_style(p, sel)),
                        Span::raw(" ".repeat(pad_count)),
                        value_span,
                    ]
                };
                card_row(frame, y, inner, view, row_type, spans, sel, focused, p);
            }
        }
        SettingsSubMenu::ModConfig => {
            let sel = selected_row == Some(SidebarRow::NavBack);
            let back_icon = nf_icons::nf!("nf-cod-arrow_left");
            let chevron = Span::styled(
                "›",
                if sel {
                    Style::default().fg(p.text).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(p.overlay0)
                },
            );
            let spans = if width <= 3 {
                vec![Span::styled(
                    format!("{back_icon}"),
                    if sel { label_style(p, true) } else { Style::default().fg(p.accent) },
                )]
            } else if width < 6 {
                vec![
                    Span::styled(
                        format!("{back_icon} "),
                        if sel { label_style(p, true) } else { Style::default().fg(p.accent) },
                    ),
                    chevron,
                ]
            } else if width < 16 {
                let avail_text = width.saturating_sub(5);
                let short: String = "Back".chars().take(avail_text).collect();
                let left = format!(" {back_icon} {short}");
                let left_len = crate::ui::text::width(&left);
                let pad_count = width.saturating_sub(left_len + 1 + 1);
                vec![
                    Span::styled(
                        left,
                        if sel { label_style(p, true) } else { Style::default().fg(p.accent) },
                    ),
                    Span::raw(" ".repeat(pad_count)),
                    chevron,
                ]
            } else {
                let left = format!(" {back_icon} Back");
                let left_w = crate::ui::text::width(&left);
                let pad_count = width.saturating_sub(left_w + 1 + 1).max(2);
                vec![
                    Span::styled(
                        left,
                        if sel {
                            label_style(p, true)
                        } else {
                            Style::default().fg(p.accent)
                        },
                    ),
                    Span::raw(" ".repeat(pad_count)),
                    chevron,
                ]
            };
            card_row(frame, y, inner, view, SidebarRow::NavBack, spans, sel, focused, p);

            for idx in 0..sidebar.mods.len() {
                let Some(item) = sidebar.mods.get(idx) else {
                    continue;
                };
                let row_type = SidebarRow::ModConfig(idx);
                let sel = selected_row == Some(row_type);
                let label = item.label.clone().unwrap_or_else(|| item.id.clone());
                let (right_sym, right_style) = if item.enabled {
                    ("[✓]", Style::default().fg(p.green).add_modifier(Modifier::BOLD))
                } else {
                    ("[✕]", Style::default().fg(p.red).add_modifier(Modifier::BOLD))
                };
                let spans = if width < 6 {
                    let initial = label.chars().next().unwrap_or('?');
                    vec![
                        Span::styled(format!("{initial}"), label_style(p, sel)),
                        Span::styled(right_sym, right_style),
                    ]
                } else if width < 16 {
                    let short_w = width.saturating_sub(5);
                    let short: String = label.chars().take(short_w).collect();
                    let left = format!(" {short}");
                    let left_w = crate::ui::text::width(&left);
                    let right_w = 3;
                    let pad_count = width.saturating_sub(left_w + right_w + 1).max(1);
                    vec![
                        Span::styled(left, label_style(p, sel)),
                        Span::raw(" ".repeat(pad_count)),
                        Span::styled(right_sym, right_style),
                    ]
                } else {
                    let left = format!(" {label}");
                    let right = format!("{right_sym} ›");
                    let left_w = crate::ui::text::width(&left);
                    let right_w = crate::ui::text::width(&right);
                    let pad_count = width.saturating_sub(left_w + right_w + 1).max(2);
                    vec![
                        Span::styled(left, label_style(p, sel)),
                        Span::raw(" ".repeat(pad_count)),
                        Span::styled(
                            right,
                            if sel {
                                Style::default().fg(p.text).add_modifier(Modifier::BOLD)
                            } else {
                                right_style
                            },
                        ),
                    ]
                };
                card_row(frame, y, inner, view, row_type, spans, sel, focused, p);
            }
        }
    }
}
