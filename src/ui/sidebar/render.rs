
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

        if box_w >= 12 {
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
                let cx = x0 + 1 + i as u16;
                if cx < x0 + box_w.saturating_sub(1) {
                    frame.buffer_mut()[(cx, y0)]
                        .set_symbol(&ch.to_string())
                        .set_style(menu_style);
                }
            }
        }

        let bottom_y = y0 + box_h - 1;
        if box_w >= 5 && box_h >= 4 {
            let selected_row = sidebar.rows.get(sidebar.selected).copied();
            let reload_icon = nf_icons::nf!("nf-cod-refresh");
            let reload_sel = selected_row == Some(super::models::SidebarRow::Reload);
            let reload_style = if reload_sel {
                Style::default()
                    .fg(p.panel_bg)
                    .bg(p.blue)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(p.blue).bg(p.sidebar_bg)
            };

            let bug_icon = nf_icons::nf!("nf-cod-bug");
            let bug_sel = selected_row == Some(super::models::SidebarRow::BugReport);
            let bug_style = if bug_sel {
                Style::default()
                    .fg(p.panel_bg)
                    .bg(p.blue)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(p.blue).bg(p.sidebar_bg)
            };

            let x_sel = selected_row == Some(super::models::SidebarRow::Twitter);
            let x_style = if x_sel {
                Style::default()
                    .fg(p.panel_bg)
                    .bg(p.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(p.accent).bg(p.sidebar_bg)
            };

            if box_w < 12 {
                let reload_start = x0 + 1;
                frame.buffer_mut()[(reload_start, bottom_y)]
                    .set_symbol(reload_icon)
                    .set_style(reload_style);
                view.zones.push(super::models::ClickZone {
                    y: bottom_y,
                    x_start: reload_start,
                    x_end: reload_start + 1,
                    row: super::models::SidebarRow::Reload,
                });

                if box_w >= 7 {
                    let bug_start = x0 + 3;
                    frame.buffer_mut()[(bug_start, bottom_y)]
                        .set_symbol(bug_icon)
                        .set_style(bug_style);
                    view.zones.push(super::models::ClickZone {
                        y: bottom_y,
                        x_start: bug_start,
                        x_end: bug_start + 1,
                        row: super::models::SidebarRow::BugReport,
                    });
                }

                if box_w >= 9 {
                    let x_start = x0 + 5;
                    frame.buffer_mut()[(x_start, bottom_y)]
                        .set_symbol("𝕏")
                        .set_style(x_style);
                    view.zones.push(super::models::ClickZone {
                        y: bottom_y,
                        x_start,
                        x_end: x_start + 1,
                        row: super::models::SidebarRow::Twitter,
                    });
                }
            } else {
                let reload_text = format!(" {reload_icon} ");
                let reload_len = crate::ui::text::width(&reload_text) as u16;
                let reload_start = x0 + 2;

                let bug_text = format!(" {bug_icon} ");
                let bug_len = crate::ui::text::width(&bug_text) as u16;
                let bug_start = reload_start + reload_len;

                for (i, ch) in reload_text.chars().enumerate() {
                    let cx = reload_start + i as u16;
                    if cx < bug_start {
                        frame.buffer_mut()[(cx, bottom_y)]
                            .set_symbol(&ch.to_string())
                            .set_style(reload_style);
                    }
                }
                view.zones.push(super::models::ClickZone {
                    y: bottom_y,
                    x_start: reload_start,
                    x_end: reload_start + reload_len,
                    row: super::models::SidebarRow::Reload,
                });

                for (i, ch) in bug_text.chars().enumerate() {
                    let cx = bug_start + i as u16;
                    if cx < x0 + box_w - 1 {
                        frame.buffer_mut()[(cx, bottom_y)]
                            .set_symbol(&ch.to_string())
                            .set_style(bug_style);
                    }
                }
                view.zones.push(super::models::ClickZone {
                    y: bottom_y,
                    x_start: bug_start,
                    x_end: bug_start + bug_len,
                    row: super::models::SidebarRow::BugReport,
                });

                let x_text = " 𝕏 ";
                let x_len = crate::ui::text::width(x_text);
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

                let icons_end = bug_start + bug_len;
                if box_w >= 22 && ver_start > icons_end + 1 {
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

    super::settings_render::render_settings_menu(
        frame,
        inner,
        width,
        &mut y,
        sidebar,
        selected_row,
        focused,
        view,
        &p,
    );

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

        let panels_h = if sidebar.panels.is_empty() { 0 } else { sidebar.panels.len() as u16 + 1 };
        let usage_reserve = if sidebar.usage.is_some() && sidebar.show_usage {
            if width < 8 { 2 } else { 3 }
        } else {
            0
        };
        let bottom_limit = inner.bottom().saturating_sub(panels_h + usage_reserve + (if width < 8 { 1 } else { 2 }));
        y += 1;
        if y < bottom_limit {
            let history_icon = nf_icons::nf!("nf-cod-history");
            let sessions_header = if width < 5 {
                Line::from(vec![
                    Span::styled(
                        format!("{history_icon} "),
                        Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
                    ),
                ])
            } else if width < 12 {
                Line::from(vec![
                    Span::styled(
                        format!("{history_icon} SESS "),
                        Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::styled(
                        format!("{history_icon} SESSIONS "),
                        Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "─".repeat(width.saturating_sub(12)),
                        Style::default().fg(p.surface1),
                    ),
                ])
            };
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
        let new_spans = if width < 5 {
            vec![
                Span::styled(
                    format!("{add_icon}"),
                    Style::default()
                        .fg(if sel_new { p.text } else { p.green })
                        .add_modifier(Modifier::BOLD),
                ),
            ]
        } else if width < 12 {
            vec![
                Span::styled(
                    format!(" {add_icon} "),
                    Style::default()
                        .fg(if sel_new { p.text } else { p.green })
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("New", label_style(&p, sel_new)),
            ]
        } else {
            let new_text = format!(" {add_icon} New Session ");
            let new_w = crate::ui::text::width(&new_text);
            let pad_total = width.saturating_sub(new_w + 1);
            let pad_left = pad_total / 2;
            let pad_right = pad_total.saturating_sub(pad_left);
            vec![
                Span::raw(" ".repeat(pad_left)),
                Span::styled(
                    format!("{add_icon} "),
                    Style::default()
                        .fg(if sel_new { p.text } else { p.green })
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("New Session", label_style(&p, sel_new)),
                Span::raw(" ".repeat(pad_right)),
            ]
        };
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
            let live = panes
                .iter()
                .any(|pane| pane.lock().map(|g| g.is_session_live(&session.id)).unwrap_or(false));
            let dot_icon = if live {
                nf_icons::nf!("nf-cod-terminal")
            } else {
                nf_icons::nf!("nf-oct-dot_fill")
            };
            let dot_style = if sel {
                Style::default().fg(p.text)
            } else if let Some(color) = sidebar.session_marks.get(&session.id) {
                Style::default().fg(crate::ui::mod_bridge::color_from_name(&p, color))
            } else if live {
                Style::default().fg(p.green)
            } else {
                Style::default().fg(p.accent)
            };

            let spans = if width < 5 {
                vec![
                    Span::styled(format!("{dot_icon}"), dot_style),
                ]
            } else if width < 16 {
                let max_chars = width.saturating_sub(4).max(1);
                let title = session_title(&session.title, max_chars);
                vec![
                    Span::styled(format!(" {dot_icon} "), dot_style),
                    Span::styled(title, Style::default().fg(p.text)),
                ]
            } else {
                let age = if session.age_short.is_empty() {
                    "·"
                } else {
                    &session.age_short
                };
                let title_max = width.saturating_sub(4 + 1 + age.chars().count());
                let formatted = session_title(&session.title, title_max);
                vec![
                    Span::styled(format!("{dot_icon} "), dot_style),
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
                ]
            };
            card_row(frame, &mut y, inner, view, row_type, spans, sel, focused, &p);
        }

        if !sidebar.sessions.is_empty() && y < bottom_limit {
            let row_type = SidebarRow::MoreSessions;
            let sel = selected_row == Some(row_type);
            let ellipsis_icon = nf_icons::nf!("nf-cod-ellipsis");
            let ellipsis_style = if sel {
                Style::default().fg(p.text)
            } else {
                Style::default().fg(p.red)
            };

            let spans = if width < 5 {
                vec![
                    Span::styled(format!("{ellipsis_icon}"), ellipsis_style),
                ]
            } else if width < 16 {
                let count = if sidebar.sessions.len() > SESSIONS_SHOWN && !sidebar.expanded {
                    format!("+{}", sidebar.sessions.len() - SESSIONS_SHOWN)
                } else {
                    "All".to_string()
                };
                vec![
                    Span::styled(format!(" {ellipsis_icon} "), ellipsis_style),
                    Span::styled(count, Style::default().fg(p.text)),
                ]
            } else {
                let label = if sidebar.sessions.len() > SESSIONS_SHOWN && !sidebar.expanded {
                    format!("+{} more · Manage", sidebar.sessions.len() - SESSIONS_SHOWN)
                } else {
                    "Manage Delete".to_string()
                };
                let label_len = label.chars().count();
                vec![
                    Span::styled(format!("{ellipsis_icon} "), ellipsis_style),
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
                ]
            };
            card_row(frame, &mut y, inner, view, row_type, spans, sel, focused, &p);
        }
    }

    if sidebar.settings_menu == SettingsSubMenu::Main && sidebar.show_usage {
        if let Some(ref usage) = sidebar.usage {
            let panels_h = if sidebar.panels.is_empty() { 0 } else { sidebar.panels.len() as u16 + 1 };
            let usage_h = if width < 8 { 2 } else { 3 };
            let mut yu = inner.bottom().saturating_sub(panels_h + usage_h);
            if yu < inner.bottom() {
                let gauge_icon = nf_icons::nf!("nf-cod-dashboard");
                let usage_header = if width < 5 {
                    Line::from(vec![
                        Span::styled(
                            format!("{gauge_icon} "),
                            Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
                        ),
                    ])
                } else if width < 12 {
                    Line::from(vec![
                        Span::styled(
                            format!("{gauge_icon} USE "),
                            Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
                        ),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(
                            format!("{gauge_icon} USAGE "),
                            Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            "─".repeat(width.saturating_sub(9)),
                            Style::default().fg(p.surface1),
                        ),
                    ])
                };
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

                let chevron_style = if sel {
                    Style::default().fg(p.text).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(p.overlay0)
                };

                let spans = if width < 6 {
                    vec![
                        Span::styled(pct_str, Style::default().fg(color).add_modifier(Modifier::BOLD)),
                    ]
                } else if width < 10 {
                    vec![
                        Span::styled(format!(" {tag} "), label_style(&p, sel)),
                        Span::styled(pct_str, Style::default().fg(color).add_modifier(Modifier::BOLD)),
                    ]
                } else if width < 16 {
                    let mut s = Vec::new();
                    s.push(Span::styled("‹", chevron_style));
                    s.push(Span::styled(format!(" {tag} "), label_style(&p, sel)));
                    let used_w = 1 + 1 + tag.len() + 1 + pct_str.len() + 1;
                    let pad_w = width.saturating_sub(used_w + 1);
                    if pad_w > 0 {
                        s.push(Span::raw(" ".repeat(pad_w)));
                    }
                    s.push(Span::styled(
                        format!("{pct_str} "),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ));
                    s.push(Span::styled("›", chevron_style));
                    s
                } else {
                    let show_info = !info.is_empty() && width >= 24;
                    let info_part = if show_info {
                        format!(" {info}")
                    } else {
                        String::new()
                    };
                    let left_len = 1 + 4;
                    let right_len = 5 + info_part.chars().count() + 1;
                    let fixed_total = left_len + right_len;
                    let bar_w = width.saturating_sub(fixed_total).max(if width >= 18 { 3 } else { 1 });
                    let (filled, empty) = crate::usage::build_ascii_bar(pct, bar_w);
                    let actual_used = left_len + bar_w + right_len;
                    let pad_count = width.saturating_sub(actual_used);

                    let mut s = vec![
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
                        s.push(Span::styled(info_part, Style::default().fg(p.overlay0)));
                    }
                    if pad_count > 0 {
                        s.push(Span::raw(" ".repeat(pad_count)));
                    }
                    s.push(Span::styled("›", chevron_style));
                    s
                };

                if width >= 12 {
                    view.zones.push(ClickZone {
                        y: yu,
                        x_start: inner.left(),
                        x_end: inner.left() + 3,
                        row: SidebarRow::UsagePrev,
                    });
                    view.zones.push(ClickZone {
                        y: yu,
                        x_start: inner.right().saturating_sub(3),
                        x_end: inner.right(),
                        row: SidebarRow::UsageNext,
                    });
                    view.zones.push(ClickZone {
                        y: yu,
                        x_start: inner.left() + 3,
                        x_end: inner.right().saturating_sub(3),
                        row: SidebarRow::UsageCarousel,
                    });
                } else {
                    view.zones.push(ClickZone {
                        y: yu,
                        x_start: inner.left(),
                        x_end: inner.right(),
                        row: SidebarRow::UsageCarousel,
                    });
                }

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

    if sidebar.settings_menu == SettingsSubMenu::Main && !sidebar.panels.is_empty() {
        let count = sidebar.panels.len() as u16;
        let yb = inner.bottom().saturating_sub(count + 1);

        let sidebars_icon = nf_icons::nf!("nf-cod-layout_sidebar_right");
        let right_header = if width < 5 {
            Line::from(vec![
                Span::styled(
                    format!("{sidebars_icon} "),
                    Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
                ),
            ])
        } else if width < 12 {
            Line::from(vec![
                Span::styled(
                    format!("{sidebars_icon} DOCKS "),
                    Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
                ),
            ])
        } else {
            Line::from(vec![
                Span::styled(
                    format!("{sidebars_icon} SIDEBARS "),
                    Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "─".repeat(width.saturating_sub(13)),
                    Style::default().fg(p.surface1),
                ),
            ])
        };
        if yb < inner.bottom() {
            frame.buffer_mut().set_line(
                inner.left() + 1,
                yb,
                &right_header,
                inner.width.saturating_sub(1),
            );
        }

        let mut yb = inner.bottom().saturating_sub(count);
        for (i, panel) in sidebar.panels.iter().enumerate() {
            if yb >= inner.bottom() {
                break;
            }
            let row_type = SidebarRow::RightSidebar(i);
            let sel = selected_row == Some(row_type);
            let is_active = sidebar.active_panel == Some(i);
            let is_open = is_active && sidebar.panel_open;
            let resolved_glyph = crate::ui::glyph::resolve_glyph(&panel.icon);
            let glyph = if resolved_glyph.is_empty() { "◆" } else { resolved_glyph };
            let status_indicator = if is_open {
                Span::styled(
                    "●",
                    Style::default().fg(p.green).add_modifier(Modifier::BOLD),
                )
            } else if is_active {
                Span::styled(
                    "○",
                    Style::default().fg(p.green),
                )
            } else {
                Span::styled(
                    "›",
                    if sel {
                        Style::default().fg(p.text).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(p.overlay0)
                    },
                )
            };
            let row_style = if is_open {
                Style::default().fg(p.text).add_modifier(Modifier::BOLD)
            } else if is_active {
                Style::default().fg(p.text)
            } else {
                label_style(&p, sel)
            };
            let spans = if width <= 3 {
                let st = if is_open {
                    Style::default().fg(p.green).add_modifier(Modifier::BOLD)
                } else if is_active {
                    Style::default().fg(p.green)
                } else {
                    label_style(&p, sel)
                };
                vec![Span::styled(format!("{glyph}"), st)]
            } else if width < 6 {
                vec![
                    Span::styled(format!("{glyph} "), row_style),
                    status_indicator,
                ]
            } else if width < 16 {
                let text_w = width.saturating_sub(5);
                let short: String = panel.title.chars().take(text_w).collect();
                let left = format!(" {glyph} {short}");
                let left_len = crate::ui::text::width(&left);
                let pad_count = width.saturating_sub(left_len + 1 + 1);
                vec![
                    Span::styled(left, row_style),
                    Span::raw(" ".repeat(pad_count)),
                    status_indicator,
                ]
            } else {
                let left = format!(" {} {} ", glyph, panel.title);
                let left_len = crate::ui::text::width(&left);
                vec![
                    Span::styled(left, row_style),
                    Span::raw(pad(width, left_len, 1)),
                    status_indicator,
                ]
            };
            card_row(frame, &mut yb, inner, view, row_type, spans, sel, focused, &p);
        }
    }
}



