
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::Frame;

use crate::theme::Palette;
use crate::ui::sidebar::models::{ClickZone, SidebarRow};
use crate::ui::sidebar::state::{session_title, LiveBlock};

pub fn render_live_block(
    frame: &mut Frame,
    inner: Rect,
    width: usize,
    y: &mut u16,
    block: &LiveBlock,
    selected_row: Option<SidebarRow>,
    focused: bool,
    view: &mut crate::ui::sidebar::models::SidebarView,
    now_ms: u128,
) -> usize {
    let p = Palette::dark();
    let mut consumed = 0usize;
    let mut cy = *y;
    let next_line = |cy: &mut u16, consumed: &mut usize, inner: Rect| -> bool {
        if *cy >= inner.bottom() {
            return false;
        }
        *cy += 1;
        *consumed += 1;
        true
    };

    if cy >= inner.bottom() {
        return consumed;
    }
    let term_prefix = if block.terminal > 0 {
        format!("{} · ", block.terminal)
    } else {
        String::new()
    };
    let header_label = if block.stalled {
        format!("{}{} PAUSED ", term_prefix, block.label)
    } else if block.done && block.aborted {
        format!("{}{} ABORTED ", term_prefix, block.label)
    } else if block.done {
        format!("{}{} COMPLETE ", term_prefix, block.label)
    } else {
        format!("{}{} LIVE ", term_prefix, block.label)
    };
    let header_label_len = crate::ui::text::width(&header_label);
    let header_fg = if block.stalled || (block.done && block.aborted) {
        p.red
    } else if block.done {
        p.green
    } else {
        p.yellow
    };
    let header = Line::from(vec![
        Span::styled(
            header_label,
            Style::default().fg(header_fg).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "─".repeat(width.saturating_sub(header_label_len + 1)),
            Style::default().fg(p.surface1),
        ),
    ]);
    frame.buffer_mut().set_line(
        inner.left() + 1,
        cy,
        &header,
        inner.width.saturating_sub(1),
    );
    if !next_line(&mut cy, &mut consumed, inner) {
        return consumed;
    }

    let spinner = ["|", "/", "-", "\\"][(now_ms as usize / 120) % 4];
    for agent in &block.agents {
        if cy >= inner.bottom() {
            break;
        }
        let (icon, icon_style) = match agent.status.as_str() {
            "active" if block.done && block.aborted => ("✕", Style::default().fg(p.red)),
            "active" if block.stalled => ("⏸", Style::default().fg(p.peach)),
            "paused" | "stalled" => ("⏸", Style::default().fg(p.peach)),
            "active" => (spinner, Style::default().fg(p.yellow)),
            "blocked" => ("!", Style::default().fg(p.red)),
            "passed" => ("✓", Style::default().fg(p.green)),
            _ => ("·", Style::default().fg(p.subtext0)),
        };
        let label_max = width.saturating_sub(7);
        let shown = session_title(&agent.label, label_max);
        let line = Line::from(vec![
            Span::styled(format!(" {} ", icon), icon_style),
            Span::styled(shown, Style::default().fg(p.text)),
        ]);
        frame.buffer_mut().set_line(
            inner.left() + 1,
            cy,
            &line,
            inner.width.saturating_sub(1),
        );
        if !next_line(&mut cy, &mut consumed, inner) {
            break;
        }
    }

    if block.stalled {
        if let Some(hint) = &block.hint {
            if cy < inner.bottom() {
                let line = Line::from(vec![
                    Span::styled(" → ", Style::default().fg(p.yellow)),
                    Span::styled(
                        hint.clone(),
                        Style::default().fg(p.yellow).add_modifier(Modifier::BOLD),
                    ),
                ]);
                frame.buffer_mut().set_line(
                    inner.left() + 1,
                    cy,
                    &line,
                    inner.width.saturating_sub(1),
                );
                if !next_line(&mut cy, &mut consumed, inner) {
                    return consumed;
                }
            }
        }
    }

    if cy >= inner.bottom() {
        return consumed;
    }
    let rule = Line::from(vec![Span::styled(
        "─".repeat(width.saturating_sub(4)),
        Style::default().fg(p.surface1),
    )]);
    frame.buffer_mut().set_line(
        inner.left() + 1,
        cy,
        &rule,
        inner.width.saturating_sub(1),
    );
    if !next_line(&mut cy, &mut consumed, inner) {
        return consumed;
    }
    if cy >= inner.bottom() {
        return consumed;
    }

    let can_resume = block.resume_command.is_some() && (!block.done || block.aborted || block.stalled);
    let has_open = block.open_path.is_some();
    let has_copy = block.copy_text.is_some();
    let dismissible = block.done || block.stalled || block.aborted;
    let term_title = if block.terminal > 0 {
        format!("Terminal {}", block.terminal)
    } else {
        "Terminal".to_string()
    };
    let term_title_len = crate::ui::text::width(" ◆ ") + crate::ui::text::width(&term_title);
    let n_icons = (can_resume as usize) + (has_open as usize) + (has_copy as usize) + (dismissible as usize);
    let gap = 2usize.min(width.saturating_sub(term_title_len + 3 * n_icons as usize + 1));

    let mut spans = vec![
        Span::styled(
            " ◆ ",
            Style::default().fg(p.blue).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            term_title.clone(),
            Style::default().fg(p.text).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" ".repeat(gap)),
    ];
    let icons_x = inner.left() + 1 + term_title_len as u16 + gap as u16;
    let mut zones: Vec<ClickZone> = Vec::new();
    let mut icon_x = icons_x;
    if can_resume {
        let sel = selected_row == Some(SidebarRow::LiveBlockResume(0));
        let style = if sel {
            Style::default()
                .fg(p.panel_bg)
                .bg(p.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.accent)
        };
        spans.push(Span::styled(
            format!(" {} ", nf_icons::nf!("nf-cod-play")),
            style,
        ));
        zones.push(ClickZone {
            y: cy,
            x_start: icon_x,
            x_end: icon_x + 3,
            row: SidebarRow::LiveBlockResume(0),
        });
        icon_x += 3;
    }
    if has_open {
        let sel = selected_row == Some(SidebarRow::LiveBlockOpen(0));
        let style = if sel {
            Style::default()
                .fg(p.panel_bg)
                .bg(p.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.green)
        };
        spans.push(Span::styled(" → ", style));
        zones.push(ClickZone {
            y: cy,
            x_start: icon_x,
            x_end: icon_x + 3,
            row: SidebarRow::LiveBlockOpen(0),
        });
        icon_x += 3;
    }
    if has_copy {
        let sel = selected_row == Some(SidebarRow::LiveBlockCopy(0));
        let style = if sel {
            Style::default()
                .fg(p.panel_bg)
                .bg(p.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.blue)
        };
        spans.push(Span::styled(" ⎘ ", style));
        zones.push(ClickZone {
            y: cy,
            x_start: icon_x,
            x_end: icon_x + 3,
            row: SidebarRow::LiveBlockCopy(0),
        });
        icon_x += 3;
    }
    if dismissible {
        let sel = selected_row == Some(SidebarRow::LiveBlockDismiss(0));
        let style = if sel {
            Style::default()
                .fg(p.panel_bg)
                .bg(p.red)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.red)
        };
        spans.push(Span::styled(
            format!(" {} ", nf_icons::nf!("nf-cod-close")),
            style,
        ));
        zones.push(ClickZone {
            y: cy,
            x_start: icon_x,
            x_end: icon_x + 3,
            row: SidebarRow::LiveBlockDismiss(0),
        });
        icon_x += 3;
    }
    let _ = icon_x;
    if n_icons == 0 {

        let dot_style = Style::default().fg(p.surface1);
        spans.push(Span::styled(" ⸱ ", dot_style));
        spans.push(Span::styled(" ⸱ ", dot_style));
        spans.push(Span::styled(" ⸱ ", dot_style));
    }

    let action_row = if can_resume {
        SidebarRow::LiveBlockResume(0)
    } else if dismissible && has_open {
        SidebarRow::LiveBlockOpen(0)
    } else if has_copy {
        SidebarRow::LiveBlockCopy(0)
    } else if dismissible {
        SidebarRow::LiveBlockDismiss(0)
    } else {
        SidebarRow::LiveBlockResume(0)
    };
    let sel = selected_row == Some(action_row);

    let mut full = vec![Span::styled(
        " ",
        Style::default().bg(if sel && focused { Palette::dark().blue } else { Palette::dark().panel_bg }),
    )];
    full.extend(spans);
    frame.buffer_mut().set_line(
        inner.left(),
        cy,
        &Line::from(full),
        inner.width.saturating_sub(1),
    );
    view.row_y.push((cy, action_row));

    view.zones.push(ClickZone {
        y: cy,
        x_start: inner.left() + 1,
        x_end: icons_x,
        row: SidebarRow::LiveBlockResume(0),
    });
    for z in zones {
        view.zones.push(z);
    }
    consumed += 1;
    *y += consumed as u16;
    consumed
}

