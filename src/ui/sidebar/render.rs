
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::theme::Palette;
use crate::ui::widget::SelectableRow;

use super::live_block::render_live_block;
use super::models::SettingsSubMenu;
use super::models::{ClickZone, SidebarRow, SidebarView, SESSIONS_SHOWN};
use super::state::{session_title, LiveBlock, Sidebar};

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

pub fn render_sidebar(
    frame: &mut ratatui::Frame,
    area: Rect,
    sidebar: &Sidebar,
    view: &mut SidebarView,
    focused: bool,
    panes: &[std::sync::Arc<std::sync::Mutex<crate::ui::pane::MuxPane>>],
) {
    let p = Palette::dark();

    if area.width < 4 || area.height < 4 {
        return;
    }

    view.row_y.clear();
    view.zones.clear();
    let bg = crate::theme::effective_bg();
    fill_area(frame, area, bg);

    let border_color = if focused {
        Palette::dark().blue
    } else {
        p.surface1
    };
    let border_style = Style::default().fg(border_color);

    let x0 = area.x;
    let y0 = area.y;
    let box_w = area.width;
    let box_h = area.height;

    if box_w >= 4 && box_h >= 4 {
        for x in x0 + 1..x0 + box_w.saturating_sub(1) {
            frame.buffer_mut()[(x, y0)]
                .set_symbol("─")
                .set_style(border_style);
            frame.buffer_mut()[(x, y0 + box_h - 1)]
                .set_symbol("─")
                .set_style(border_style);
        }
        let mid_y = y0 + box_h / 2;
        for y in y0 + 1..y0 + box_h.saturating_sub(1) {
            frame.buffer_mut()[(x0, y)]
                .set_symbol("│")
                .set_style(border_style);
            let is_grip = y >= mid_y.saturating_sub(1) && y <= mid_y + 1;
            let (sym, style) = if is_grip {
                ("║", Style::default().fg(p.accent).add_modifier(Modifier::BOLD))
            } else {
                ("│", border_style)
            };
            frame.buffer_mut()[(x0 + box_w - 1, y)]
                .set_symbol(sym)
                .set_style(style);
        }
        frame.buffer_mut()[(x0, y0)]
            .set_symbol("╭")
            .set_style(border_style);
        frame.buffer_mut()[(x0 + box_w - 1, y0)]
            .set_symbol("╮")
            .set_style(border_style);
        frame.buffer_mut()[(x0, y0 + box_h - 1)]
            .set_symbol("╰")
            .set_style(border_style);
        frame.buffer_mut()[(x0 + box_w - 1, y0 + box_h - 1)]
            .set_symbol("╯")
            .set_style(border_style);

        let gear_icon = nf_icons::nf!("nf-cod-settings_gear");
        let menu_title = format!(" {gear_icon} MENU ");
        let menu_style = Style::default()
            .fg(if focused {
                Palette::dark().blue
            } else {
                p.blue
            })
            .add_modifier(Modifier::BOLD);
        for (i, ch) in menu_title.chars().enumerate() {
            let cx = x0 + 2 + i as u16;
            if cx < x0 + box_w - 2 {
                frame.buffer_mut()[(cx, y0)]
                    .set_symbol(&ch.to_string())
                    .set_style(menu_style);
            }
        }

        let bottom_y = y0 + box_h - 1;
        if box_w >= 12 && box_h >= 6 {
            let x_text = " 𝕏 ";
            let x_len = crate::ui::text::width(x_text);
            let x_style = Style::default()
                .fg(p.panel_bg)
                .bg(p.accent)
                .add_modifier(Modifier::BOLD);
            let x_start = x0 + box_w.saturating_sub(x_len as u16 + 2);
            for (i, ch) in x_text.chars().enumerate() {
                let cx = x_start + i as u16;
                if cx < x0 + box_w - 1 {
                    frame.buffer_mut()[(cx, bottom_y)]
                        .set_symbol(&ch.to_string())
                        .set_style(x_style);
                }
            }
            view.zones.push(super::models::ClickZone {
                y: bottom_y,
                x_start,
                x_end: x_start + x_len as u16,
                row: super::models::SidebarRow::Twitter,
            });

            let has_update = sidebar.available_update.is_some();
            let ver_text = if has_update {
                " ! NEW ".to_string()
            } else {
                format!(" v{} ", env!("CARGO_PKG_VERSION"))
            };
            let ver_len = crate::ui::text::width(&ver_text);
            let ver_style = if has_update {
                Style::default()
                    .fg(p.green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(p.overlay0)
            };
            let ver_start = x_start.saturating_sub(ver_len as u16);

            if box_w >= 22 && ver_start > x0 + 8 {
                for (i, ch) in ver_text.chars().enumerate() {
                    let cx = ver_start + i as u16;
                    if cx < x_start {
                        frame.buffer_mut()[(cx, bottom_y)]
                            .set_symbol(&ch.to_string())
                            .set_style(ver_style);
                    }
                }
                if has_update {
                    view.zones.push(super::models::ClickZone {
                        y: bottom_y,
                        x_start: ver_start,
                        x_end: x_start,
                        row: super::models::SidebarRow::Update,
                    });
                }
            }

            let c_text = " © 2026 ";
            let c_len = crate::ui::text::width(c_text);
            let c_style = Style::default().fg(p.overlay0);
            let max_c_x = if box_w >= 22 && ver_start > x0 + 8 {
                ver_start
            } else {
                x_start
            };

            if max_c_x >= x0 + 2 + c_len as u16 {
                for (i, ch) in c_text.chars().enumerate() {
                    let cx = x0 + 2 + i as u16;
                    if cx < max_c_x {
                        frame.buffer_mut()[(cx, bottom_y)]
                            .set_symbol(&ch.to_string())
                            .set_style(c_style);
                    }
                }
            }
        }
    }

    let inner = Rect::new(
        area.x + 1,
        area.y + 1,
        area.width.saturating_sub(2),
        box_h.saturating_sub(2),
    );
    let width = inner.width as usize;
    let mut y = inner.top();

    fn label_style(p: &Palette, selected: bool) -> Style {
        if selected {
            Style::default().fg(p.text).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.subtext0)
        }
    }

    fn toggle_span(p: &Palette, on: bool, _selected: bool) -> Span<'_> {
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

    let selected_row = sidebar.rows.get(sidebar.selected).copied();

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
    ) -> () {
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
            frame.render_widget(SelectableRow::new(spans, selected, &p), row_area(inner, *y));
            *y += 1;
        }
    }

    match sidebar.settings_menu {
        SettingsSubMenu::Main => {
            for (row_type, icon, label) in [
                (SidebarRow::NavPreferences, nf_icons::nf!("nf-cod-settings_gear"), "Preferences"),
                (SidebarRow::NavModConfig, nf_icons::nf!("nf-cod-extensions"), "Mod Config"),
                (SidebarRow::NavAIPrefs, nf_icons::nf!("nf-cod-sparkle"), "AI Prefs"),
            ] {
                let sel = selected_row == Some(row_type);
                let left = format!(" {} {} ", icon, label);
                let left_len = crate::ui::text::width(&left);
                let spans = vec![
                    Span::styled(left, label_style(&p, sel)),
                    Span::raw(pad(width, left_len, 1)),
                    Span::styled(
                        "›",
                        if sel {
                            Style::default().fg(p.text).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(p.overlay0)
                        },
                    ),
                ];
                card_row(frame, &mut y, inner, view, row_type, spans, sel, focused, &p);
            }
            let sel = selected_row == Some(SidebarRow::Reload);
            let reload_icon = nf_icons::nf!("nf-cod-refresh");
            let reload_text = format!(" {} Reload Process ", reload_icon);
            let reload_len = crate::ui::text::width(&reload_text);
            card_row(
                frame,
                &mut y,
                inner,
                view,
                SidebarRow::Reload,
                vec![
                    Span::styled(reload_text, label_style(&p, sel)),
                    Span::raw(pad(width, reload_len, 1)),
                    Span::styled(
                        "›",
                        if sel {
                            Style::default().fg(p.text).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(p.overlay0)
                        },
                    ),
                ],
                sel,
                focused,
                &p,
            );
        }
        SettingsSubMenu::Preferences => {
            let sel = selected_row == Some(SidebarRow::NavBack);
            let left = format!(" {} Back", nf_icons::nf!("nf-cod-arrow_left"));
            let left_w = crate::ui::text::width(&left);
            let pad_count = width.saturating_sub(left_w + 1 + 1).max(2);
            card_row(
                frame,
                &mut y,
                inner,
                view,
                SidebarRow::NavBack,
                vec![
                    Span::styled(
                        left,
                        if sel {
                            label_style(&p, true)
                        } else {
                            Style::default().fg(p.accent)
                        },
                    ),
                    Span::raw(" ".repeat(pad_count)),
                    Span::styled(
                        "›",
                        if sel {
                            Style::default().fg(p.text).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(p.overlay0)
                        },
                    ),
                ],
                sel,
                focused,
                &p,
            );

            let sel = selected_row == Some(SidebarRow::PrefFullConfig);
            let left = " Full Config".to_string();
            let right = "edit ›";
            let left_w = crate::ui::text::width(&left);
            let right_w = crate::ui::text::width(right);
            let pad_count = width.saturating_sub(left_w + right_w + 1).max(2);
            let spans = vec![
                Span::styled(left, label_style(&p, sel)),
                Span::raw(" ".repeat(pad_count)),
                Span::styled(
                    right,
                    if sel {
                        Style::default().fg(p.text).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(p.yellow)
                    },
                ),
            ];
            card_row(frame, &mut y, inner, view, SidebarRow::PrefFullConfig, spans, sel, focused, &p);

            let sel = selected_row == Some(SidebarRow::PrefAutoRetry);
            let left = " Error Recovery".to_string();
            let right = if sidebar.auto_retry_enabled { "ON ›" } else { "OFF ›" };
            let left_w = crate::ui::text::width(&left);
            let right_w = crate::ui::text::width(right);
            let pad_count = width.saturating_sub(left_w + right_w + 1).max(2);
            let spans = vec![
                Span::styled(left, label_style(&p, sel)),
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
            ];
            card_row(frame, &mut y, inner, view, SidebarRow::PrefAutoRetry, spans, sel, focused, &p);

            let sel = selected_row == Some(SidebarRow::PrefSkills);
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
            let spans = vec![
                Span::styled(left, label_style(&p, sel)),
                Span::raw(" ".repeat(pad_count)),
                Span::styled(
                    right,
                    if sel {
                        Style::default().fg(p.text).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(right_color)
                    },
                ),
            ];
            card_row(frame, &mut y, inner, view, SidebarRow::PrefSkills, spans, sel, focused, &p);

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

            for (row_type, label, on) in [
                (SidebarRow::PrefSkillInjection, "Skill Injection", Some(sidebar.skill_injection)),
                (SidebarRow::PrefYolo, "YOLO Mode", Some(sidebar.yolo_mode)),
                (SidebarRow::PrefShowUsage, "Show Usage", Some(sidebar.show_usage)),
                (SidebarRow::PrefSounds, "Sound Alerts", Some(sidebar.sound_notifications)),
            ] {
                let sel = selected_row == Some(row_type);
                let value_span = if let Some(on) = on {
                    toggle_span(&p, on, sel)
                } else {
                    Span::raw("")
                };
                let left = format!(" {label}");
                let left_w = crate::ui::text::width(&left);
                let right_w = 3;
                let pad_count = width.saturating_sub(left_w + right_w + 1).max(2);
                let spans = vec![
                    Span::styled(left, label_style(&p, sel)),
                    Span::raw(" ".repeat(pad_count)),
                    value_span,
                ];
                card_row(frame, &mut y, inner, view, row_type, spans, sel, focused, &p);
            }
        }
        SettingsSubMenu::ModConfig => {
            let sel = selected_row == Some(SidebarRow::NavBack);
            let left = format!(" {} Back", nf_icons::nf!("nf-cod-arrow_left"));
            let left_w = crate::ui::text::width(&left);
            let pad_count = width.saturating_sub(left_w + 1 + 1).max(2);
            card_row(
                frame,
                &mut y,
                inner,
                view,
                SidebarRow::NavBack,
                vec![
                    Span::styled(
                        left,
                        if sel {
                            label_style(&p, true)
                        } else {
                            Style::default().fg(p.accent)
                        },
                    ),
                    Span::raw(" ".repeat(pad_count)),
                    Span::styled(
                        "›",
                        if sel {
                            Style::default().fg(p.text).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(p.overlay0)
                        },
                    ),
                ],
                sel,
                focused,
                &p,
            );

            for idx in 0..sidebar.mods.len() {
                let Some(item) = sidebar.mods.get(idx) else {
                    continue;
                };
                let row_type = SidebarRow::ModConfig(idx);
                let sel = selected_row == Some(row_type);
                let label = item.label.clone().unwrap_or_else(|| item.id.clone());
                let left = format!(" {label}");
                let right = if item.enabled {
                    "[✓] ›".to_string()
                } else {
                    "[✕] ›".to_string()
                };
                let right_style = if sel {
                    Style::default().fg(p.text).add_modifier(Modifier::BOLD)
                } else if item.enabled {
                    Style::default().fg(p.green).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(p.red).add_modifier(Modifier::BOLD)
                };
                let left_w = crate::ui::text::width(&left);
                let right_w = crate::ui::text::width(&right);
                let pad_count = width.saturating_sub(left_w + right_w + 1).max(2);
                let spans = vec![
                    Span::styled(left, label_style(&p, sel)),
                    Span::raw(" ".repeat(pad_count)),
                    Span::styled(right, right_style),
                ];
                card_row(frame, &mut y, inner, view, row_type, spans, sel, focused, &p);
            }
        }
    }

    if sidebar.settings_menu == SettingsSubMenu::Main {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        for block in &sidebar.live_blocks {
            if LiveBlock::is_dismissed(&block.id) {
                continue;
            }
            let mut resolved_block = block.clone();
            if resolved_block.terminal == 0 && !resolved_block.session_id.is_empty() {
                if let Some(idx) = panes.iter().position(|p| {
                    p.lock()
                        .map(|g| g.state.session_id.as_deref() == Some(resolved_block.session_id.as_str()))
                        .unwrap_or(false)
                }) {
                    resolved_block.terminal = idx + 1;
                }
            }
            if y >= inner.bottom() {
                break;
            }
            y += 1;
            let used = render_live_block(
                frame,
                inner,
                width,
                &mut y,
                &resolved_block,
                selected_row,
                focused,
                view,
                now_ms,
            );
            if used == 0 {
                break;
            }
        }
    }

    if sidebar.settings_menu == SettingsSubMenu::Main {

        let bottom_limit = if sidebar.usage.is_some() && sidebar.show_usage {
            inner.bottom().saturating_sub(6)
        } else {
            inner.bottom().saturating_sub(3)
        };
        y += 1;
        if y < bottom_limit {
            let history_icon = nf_icons::nf!("nf-cod-history");
            let sessions_header = Line::from(vec![
                Span::styled(
                    format!("{history_icon} SESSIONS "),
                    Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "─".repeat(width.saturating_sub(12)),
                    Style::default().fg(p.surface1),
                ),
            ]);
            frame.buffer_mut().set_line(
                inner.left() + 1,
                y,
                &sessions_header,
                inner.width.saturating_sub(1),
            );
            y += 1;
        }

        let sel_new = selected_row == Some(SidebarRow::NewSession);
        let add_icon = nf_icons::nf!("nf-cod-add");
        let new_text = format!(" {add_icon} New Session ");
        let new_w = crate::ui::text::width(&new_text);
        let pad_total = width.saturating_sub(new_w + 1);
        let pad_left = pad_total / 2;
        let pad_right = pad_total.saturating_sub(pad_left);
        let new_spans = vec![
            Span::raw(" ".repeat(pad_left)),
            Span::styled(
                format!("{add_icon} "),
                Style::default()
                    .fg(if sel_new { p.text } else { p.green })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("New Session", label_style(&p, sel_new)),
            Span::raw(" ".repeat(pad_right)),
        ];
        if y < bottom_limit {
            card_row(frame, &mut y, inner, view, SidebarRow::NewSession, new_spans, sel_new, focused, &p);
        }

        let shown = if sidebar.expanded {
            sidebar.sessions.len()
        } else {
            sidebar.sessions.len().min(SESSIONS_SHOWN)
        };

        for i in 0..shown {
            if y >= bottom_limit {
                break;
            }
            let Some(session) = sidebar.sessions.get(i) else {
                continue;
            };
            let row_type = SidebarRow::Session(i);
            let sel = selected_row == Some(row_type);
            let age = if session.age_short.is_empty() {
                "·"
            } else {
                &session.age_short
            };
            let title_max = width.saturating_sub(4 + 1 + age.chars().count());
            let formatted = session_title(&session.title, title_max);
            let dot_icon = nf_icons::nf!("nf-oct-dot_fill");

            let spans = vec![
                Span::styled(
                    format!("{dot_icon} "),
                    if sel {
                        Style::default().fg(p.text)
                    } else {
                        Style::default().fg(p.accent)
                    },
                ),
                Span::styled(formatted, Style::default().fg(p.text)),
                Span::raw(" "),
                Span::styled(
                    age.to_string(),
                    if sel {
                        Style::default().fg(p.text)
                    } else {
                        Style::default().fg(p.overlay0)
                    },
                ),
            ];
            card_row(frame, &mut y, inner, view, row_type, spans, sel, focused, &p);
        }

        if !sidebar.sessions.is_empty() && y < bottom_limit {
            let row_type = SidebarRow::MoreSessions;
            let sel = selected_row == Some(row_type);
            let label = if sidebar.sessions.len() > SESSIONS_SHOWN && !sidebar.expanded {
                format!("+{} more · Manage", sidebar.sessions.len() - SESSIONS_SHOWN)
            } else {
                "Manage Delete".to_string()
            };
            let label_len = label.chars().count();
            let ellipsis_icon = nf_icons::nf!("nf-cod-ellipsis");
            let spans = vec![
                Span::styled(
                    format!("{ellipsis_icon} "),
                    if sel {
                        Style::default().fg(p.text)
                    } else {
                        Style::default().fg(p.red)
                    },
                ),
                Span::styled(
                    label,
                    Style::default()
                        .fg(p.text)
                        .add_modifier(Modifier::UNDERLINED),
                ),
                Span::styled(
                    format!(" ─{}", "─".repeat(width.saturating_sub(label_len + 4))),
                    Style::default().fg(p.surface1),
                ),
            ];
            card_row(frame, &mut y, inner, view, row_type, spans, sel, focused, &p);
        }
    }

    if sidebar.settings_menu == SettingsSubMenu::Main && sidebar.show_usage {
        if let Some(ref usage) = sidebar.usage {
            let mut yu = inner.bottom().saturating_sub(6);
            if yu < inner.bottom() {
                let gauge_icon = nf_icons::nf!("nf-cod-dashboard");
                let usage_header = Line::from(vec![
                    Span::styled(
                        format!("{gauge_icon} USAGE "),
                        Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "─".repeat(width.saturating_sub(9)),
                        Style::default().fg(p.surface1),
                    ),
                ]);
                frame.buffer_mut().set_line(
                    inner.left() + 1,
                    yu,
                    &usage_header,
                    inner.width.saturating_sub(1),
                );
                yu += 1;

                let sel = selected_row == Some(SidebarRow::UsageCarousel);
                let tab = sidebar.usage_tab % 3;

                let (tag, pct, info) = match tab {
                    0 => {
                        let p_val = usage.five_hour_percent();
                        let reset = usage
                            .five_hour
                            .as_ref()
                            .map(|w| crate::usage::format_duration_from_now(w.reset_at))
                            .unwrap_or_default();
                        let info_str = if !reset.is_empty() && reset != "now" {
                            format!(" {reset}")
                        } else {
                            String::new()
                        };
                        ("5h", p_val, info_str)
                    }
                    1 => {
                        let p_val = usage.weekly_percent();
                        let reset = usage
                            .weekly
                            .as_ref()
                            .map(|w| crate::usage::format_duration_from_now(w.reset_at))
                            .unwrap_or_default();
                        let info_str = if !reset.is_empty() && reset != "now" {
                            format!(" {reset}")
                        } else {
                            String::new()
                        };
                        ("Wk", p_val, info_str)
                    }
                    _ => {
                        let p_val = usage.monthly_percent();
                        let info_str = format!(" ${:.0}", usage.monthly_remaining);
                        ("Mo", p_val, info_str)
                    }
                };

                let color = crate::usage::get_usage_color(pct, &p);
                let pct_str = format!("{:.0}%", pct);
                let info_part = if info.is_empty() {
                    String::new()
                } else {
                    format!(" {info}")
                };
                let left_len = 1 + 1 + tag.len() + 1;
                let right_len = 1 + pct_str.len() + info_part.chars().count() + 1 + 1;
                let fixed_total = 1 + left_len + right_len;
                let bar_w = width.saturating_sub(fixed_total).max(4);
                let (filled, empty) = crate::usage::build_ascii_bar(pct, bar_w);
                let pad_count = width.saturating_sub(fixed_total + bar_w);

                let chevron_style = if sel {
                    Style::default().fg(p.text).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(p.overlay0)
                };

                let mut spans = vec![
                    Span::styled("‹", chevron_style),
                    Span::styled(format!(" {tag} "), label_style(&p, sel)),
                    Span::styled(filled, Style::default().fg(color).add_modifier(Modifier::BOLD)),
                    Span::styled(empty, Style::default().fg(p.overlay1)),
                    Span::styled(
                        format!(" {pct_str}"),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                ];
                if !info_part.is_empty() {
                    spans.push(Span::styled(info_part, Style::default().fg(p.overlay0)));
                }
                spans.push(Span::raw(" ".repeat(pad_count + 1)));
                spans.push(Span::styled("›", chevron_style));

                view.zones.push(ClickZone {
                    y: yu,
                    x_start: inner.left(),
                    x_end: inner.left() + 4,
                    row: SidebarRow::UsagePrev,
                });
                view.zones.push(ClickZone {
                    y: yu,
                    x_start: inner.right().saturating_sub(4),
                    x_end: inner.right(),
                    row: SidebarRow::UsageNext,
                });
                view.zones.push(ClickZone {
                    y: yu,
                    x_start: inner.left() + 4,
                    x_end: inner.right().saturating_sub(4),
                    row: SidebarRow::UsageCarousel,
                });

                card_row(
                    frame,
                    &mut yu,
                    inner,
                    view,
                    SidebarRow::UsageCarousel,
                    spans,
                    sel,
                    focused,
                    &p,
                );
            }
        }
    }

    if sidebar.settings_menu == SettingsSubMenu::Main {

        let yb = inner.bottom().saturating_sub(3);

        let sidebars_icon = nf_icons::nf!("nf-cod-layout_sidebar_right");
        let right_header = Line::from(vec![
            Span::styled(
                format!("{sidebars_icon} SIDEBARS "),
                Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "─".repeat(width.saturating_sub(13)),
                Style::default().fg(p.surface1),
            ),
        ]);
        if yb < inner.bottom() {
            frame.buffer_mut().set_line(
                inner.left() + 1,
                yb,
                &right_header,
                inner.width.saturating_sub(1),
            );
        }

        let mut yb = inner.bottom().saturating_sub(2);
        for (i, (glyph, label)) in super::models::RIGHT_SIDEBARS.iter().enumerate() {
            if yb >= inner.bottom() {
                break;
            }
            let row_type = SidebarRow::RightSidebar(i);
            let sel = selected_row == Some(row_type);
            let left = format!(" {} {} ", glyph, label);
            let left_len = crate::ui::text::width(&left);
            let mut spans = vec![Span::styled(left, label_style(&p, sel))];

            if super::models::RIGHT_SIDEBARS.len() > 1 {
                spans.push(Span::raw(pad(width, left_len, 1)));
                spans.push(Span::styled(
                    "›",
                    if sel {
                        Style::default().fg(p.text).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(p.overlay0)
                    },
                ));
            }
            card_row(frame, &mut yb, inner, view, row_type, spans, sel, focused, &p);
        }
    }
}

