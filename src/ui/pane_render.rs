
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::TermMode;
use alacritty_terminal::vte::ansi::NamedColor;

use crate::theme::Palette;

use super::pane::{MuxPane, to_ratatui_color};

pub fn format_tokens(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.1}B", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn fmt_tokens_2(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.2}B", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.2}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.2}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn spinner_frame() -> String {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as usize)
        .unwrap_or(0);
    const W: usize = 4;
    let phase = (now_ms / 90) % (W + 1);
    let mut out = String::from("[");
    for i in 0..W {
        out.push(if i < phase { '=' } else { '-' });
    }
    out.push(']');
    out
}

pub fn render_pane_chrome(
    frame: &mut ratatui::Frame,
    area: Rect,
    title: &str,
    focused: bool,
) -> Rect {
    let p = Palette::dark();
    let buf = frame.buffer_mut();
    let color = if focused { p.accent } else { p.overlay0 };
    let border = Style::default().fg(color);

    if area.width < 2 || area.height < 2 {
        return area;
    }
    let last_x = area.x + area.width - 1;
    let last_y = area.y + area.height - 1;

    for x in area.x..=last_x {
        buf[(x, area.y)].set_symbol("─").set_style(border);
    }

    for y in area.y..=last_y {
        buf[(area.x, y)].set_symbol("│").set_style(border);
        buf[(last_x, y)].set_symbol("│").set_style(border);
    }

    buf[(area.x, area.y)].set_symbol("╭").set_style(border);
    buf[(last_x, area.y)].set_symbol("╮").set_style(border);
    buf[(area.x, last_y)].set_symbol("╰").set_style(border);
    buf[(last_x, last_y)].set_symbol("╯").set_style(border);

    if !title.is_empty() && title != "terminal" && !title.starts_with("Terminal") {
        let title_style = if focused {
            Style::default().fg(p.accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.overlay0)
        };
        let t = format!(" {} ", title);
        let mut x = area.x + 1;
        for ch in t.chars() {
            if x >= last_x {
                break;
            }
            buf[(x, area.y)]
                .set_symbol(&ch.to_string())
                .set_style(title_style);
            x += 1;
        }
    }

    Rect::new(
        area.x + 1,
        area.y + 1,
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    )
}

pub fn render_pane(
    frame: &mut ratatui::Frame,
    area: Rect,
    pane: &mut MuxPane,
    mod_segments: &[crate::ui::mod_bridge::ModSegment],
    mod_context: Option<&crate::ui::mod_bridge::ModContextUsage>,
    bridge_stale: bool,
    show_cost_bar: bool,
    mod_known: bool,
) {
    if area.width < 3 || area.height < 3 {
        return;
    }

    super::pane::poll_metrics(pane);

    let title = if pane.state.title.is_empty() {
        "terminal".to_string()
    } else {
        pane.state.title.clone()
    };
    let inner = render_pane_chrome(frame, area, &title, true);

    let gutter = 1u16;
    let content_area = if inner.width > gutter + 1 {
        Rect::new(inner.x, inner.y, inner.width - gutter, inner.height)
    } else {
        inner
    };

    if content_area.width == 0 || content_area.height == 0 {
        return;
    }

    let bh = crate::ui::banner::banner_height(pane, content_area);
    let pty_h = if show_cost_bar && content_area.height >= 4 {
        content_area.height - 1
    } else {
        content_area.height
    }.max(1);
    pane.resize(content_area.width, pty_h);
    if pane.term.columns() != content_area.width as usize
        || pane.term.screen_lines() != pty_h as usize
    {
        crate::ipc::log_append(
            "resize.log",
            &format!(
                "MISMATCH render: engine=({},{}) wanted=({},{})",
                pane.term.columns(),
                pane.term.screen_lines(),
                content_area.width,
                pty_h
            ),
        );
    }

    let render_status_bar = |frame: &mut ratatui::Frame| {
    if content_area.height >= 4 && show_cost_bar {
        let base_bg = ratatui::style::Color::Rgb(0, 0, 0);
        let p = Palette::dark();
        let mut bar = crate::ui::widgets::status_bar::StatusBarWidget::new();

        if mod_segments.is_empty() && bridge_stale {
            bar.left_segments
                .push(crate::ui::widgets::status_bar::StatusSegment {
                    text: spinner_frame(),
                    fg: p.blue,
                    bg: base_bg,
                    bold: false,
                });
        }
        for seg in mod_segments {
            bar.left_segments
                .push(crate::ui::widgets::status_bar::StatusSegment {
                    text: seg.text.clone(),
                    fg: crate::ui::mod_bridge::color_from_name(&p, &seg.color),
                    bg: base_bg,
                    bold: seg.bold,
                });
        }

        if bridge_stale {
            bar.left_segments.clear();
            bar.left_segments
                .push(crate::ui::widgets::status_bar::StatusSegment {
                    text: spinner_frame(),
                    fg: p.blue,
                    bg: base_bg,
                    bold: false,
                });
            bar.right_segments.clear();
            bar.right_segments
                .push(crate::ui::widgets::status_bar::StatusSegment {
                    text: spinner_frame(),
                    fg: p.blue,
                    bg: base_bg,
                    bold: false,
                });
        }

        if !bridge_stale {
            if let Some(ctx) = mod_context {
                let pct = ctx.pct.clamp(0.0, 1.0);
                let color = if pct >= 0.8 {
                    p.red
                } else if pct >= 0.6 {
                    p.yellow
                } else if pct >= 0.35 {
                    p.green
                } else {
                    p.blue
                };
                let text = format!("{} / {}", fmt_tokens_2(ctx.used), fmt_tokens_2(ctx.max));
                bar.right_segments
                    .push(crate::ui::widgets::status_bar::StatusSegment {
                        text,
                        fg: color,
                        bg: base_bg,
                        bold: true,
                    });
            } else if mod_known {

                bar.right_segments
                    .push(crate::ui::widgets::status_bar::StatusSegment {
                        text: spinner_frame(),
                        fg: p.blue,
                        bg: base_bg,
                        bold: false,
                    });
            }
        }

        let bar_area = Rect::new(area.x, area.bottom() - 1, area.width, 1);
        bar.render(frame, bar_area, base_bg);
    }
    };
    render_status_bar(frame);

    if pane.state.loading {
        let pal = Palette::dark();
        let buf = frame.buffer_mut();
        let spaces = " ".repeat(content_area.width as usize);
        for y in content_area.top()..content_area.bottom() {
            buf.set_string(
                content_area.left(),
                y,
                &spaces,
                Style::default().bg(ratatui::style::Color::Rgb(0, 0, 0)),
            );
        }

        let spin_msg = format!("{} connecting to session...", spinner_frame());
        let msg_len = spin_msg.chars().count() as u16;
        let cx = content_area.left() + content_area.width.saturating_sub(msg_len) / 2;
        let cy = content_area.top() + content_area.height / 2;
        let spin_style = Style::default().fg(pal.accent).bg(pal.surface0);
        for (i, ch) in spin_msg.chars().enumerate() {
            let gx = cx + i as u16;
            if gx < content_area.right() && cy < content_area.bottom() {
                buf[(gx, cy)]
                    .set_symbol(&ch.to_string())
                    .set_style(spin_style);
            }
        }
        return;
    }

    let at_prompt_idle = !pane.is_busy() && pane.state.has_user_prompted;

    {
        let total = pane.term.total_lines();
        let viewport = pane.term.screen_lines().max(1);
        let max = total.saturating_sub(viewport);
        let raw = pane.term.grid().display_offset();
        if raw > max {
            pane.scroll_display(-((raw - max) as i32));
        }
    }

    let content = pane.term.renderable_content();
    let buf = frame.buffer_mut();

    let base_bg = ratatui::style::Color::Rgb(0, 0, 0);

    {
        let width = content_area.width as usize;
        let spaces = " ".repeat(width);
        for y in content_area.top()..content_area.bottom() {
            buf.set_string(
                content_area.left(),
                y,
                &spaces,
                Style::default().bg(base_bg),
            );
        }
    }

    let metrics = pane.scroll_metrics();
    let offset = content.display_offset.min(metrics.max_offset_from_bottom);

    let cell_in_selection = |x: u16, y: u16| -> bool {
        let (vx, vy) = (
            x.saturating_sub(content_area.left()),
            y.saturating_sub(content_area.top()),
        );
        pane.state
            .selection
            .as_ref()
            .map(|s| s.contains(vy, vx, metrics))
            .unwrap_or(false)
    };

    let min_line = -(offset as i32);
    let max_line = min_line + pty_h as i32;

    let cursor_point = content.cursor.point;
    let term_mode = content.mode;

    for item in content.display_iter {
        let line_val = item.point.line.0;
        if line_val < min_line || line_val >= max_line {
            continue;
        }
        let Some(vp) = alacritty_terminal::term::point_to_viewport(offset, item.point) else {
            continue;
        };
        let (x, y) = (vp.column.0 as u16, vp.line as u16);
        if x >= content_area.width || y >= pty_h {
            continue;
        }

        if bh > 0 && y < bh {
            continue;
        }

        let cell = item.cell;
        let is_selected = cell_in_selection(content_area.left() + x, content_area.top() + y);

        if cell.c.is_control() {
            continue;
        }

        if cell.c == ' ' && !cell.flags.intersects(Flags::WIDE_CHAR) && !is_selected {
            if cell.bg == alacritty_terminal::vte::ansi::Color::Named(NamedColor::Background)
                && cell.fg == alacritty_terminal::vte::ansi::Color::Named(NamedColor::Foreground)
                && !cell.flags.intersects(Flags::INVERSE)
            {
                continue;
            }
        }
        if cell.flags.intersects(Flags::WIDE_CHAR_SPACER) {
            continue;
        }

        let mut style = Style::default().bg(base_bg);
        if cell.fg != alacritty_terminal::vte::ansi::Color::Named(NamedColor::Foreground) {
            style = style.fg(to_ratatui_color(&cell.fg, ratatui::style::Color::White));
        }

        if cell.bg != alacritty_terminal::vte::ansi::Color::Named(NamedColor::Background) {
            let shell_bg = to_ratatui_color(&cell.bg, base_bg);
            if shell_bg != base_bg {
                style = style.bg(shell_bg);
            }
        }
        let flags = cell.flags;
        if flags.intersects(Flags::BOLD) {
            style = style.add_modifier(Modifier::BOLD);
        }
        if flags.intersects(Flags::ITALIC) {
            style = style.add_modifier(Modifier::ITALIC);
        }
        if flags.intersects(Flags::UNDERLINE) {
            style = style.add_modifier(Modifier::UNDERLINED);
        }
        if flags.intersects(Flags::STRIKEOUT) {
            style = style.add_modifier(Modifier::CROSSED_OUT);
        }
        if flags.intersects(Flags::INVERSE) {
            style = style.add_modifier(Modifier::REVERSED);
        }
        if flags.intersects(Flags::DIM) {

            style = style.add_modifier(Modifier::DIM);
        }

        if is_selected {
            let sel_pal = Palette::dark();
            style = style
                .bg(sel_pal.blue)
                .fg(sel_pal.text)
                .add_modifier(Modifier::BOLD);
        }

        let gx = content_area.left() + x;
        let gy = content_area.top() + y;
        let (bx, by, bw, bheight) = (
            buf.area().x,
            buf.area().y,
            buf.area().width,
            buf.area().height,
        );
        if gx >= bx && gx < bx + bw && gy >= by && gy < by + bheight {
            let tcell = &mut buf[(gx, gy)];

            let same_glyph = tcell.symbol().chars().next() == Some(cell.c);
            if !same_glyph || tcell.style() != style {
                tcell.set_char(cell.c);
                tcell.set_style(style);
            }
        }
    }

    if pane
        .state
        .selection
        .as_ref()
        .map(|s| s.is_visible())
        .unwrap_or(false)
    {
        let sel_pal = Palette::dark();
        let sel_bg = Style::default().bg(sel_pal.blue).fg(sel_pal.text);
        let (bx, by, bw, bheight) = (
            buf.area().x,
            buf.area().y,
            buf.area().width,
            buf.area().height,
        );
        for y in 0..pty_h {
            let gy = content_area.top() + y;
            if gy < by || gy >= by + bheight {
                continue;
            }
            for x in 0..content_area.width {
                let gx = content_area.left() + x;
                if gx < bx || gx >= bx + bw {
                    continue;
                }
                if cell_in_selection(gx, gy) {
                    let cell = &mut buf[(gx, gy)];
                    if cell.style() != sel_bg
                        && (cell.symbol() == " " || cell.style().bg == Some(base_bg))
                    {
                        cell.set_style(sel_bg);
                    }
                }
            }
        }
    }

    let show_cursor = !at_prompt_idle && term_mode.contains(TermMode::SHOW_CURSOR);
    if show_cursor {
        if let Some(vp) = alacritty_terminal::term::point_to_viewport(offset, cursor_point) {
            let (cx, cy) = (vp.column.0 as u16, vp.line as u16);
            if cx < content_area.width && cy < pty_h && (bh == 0 || cy >= bh) {
                let gx = content_area.left() + cx;
                let gy = content_area.top() + cy;
                let (bx, by, bw, buf_h) = (
                    buf.area().x,
                    buf.area().y,
                    buf.area().width,
                    buf.area().height,
                );
                if gx >= bx && gx < bx + bw && gy >= by && gy < by + buf_h {
                    let tcell = &mut buf[(gx, gy)];
                    let pal = Palette::dark();

                    let cell_bg = tcell.style().bg.unwrap_or(ratatui::style::Color::Rgb(0, 0, 0));
                    let glyph = if tcell.symbol() == " " {
                        pal.text
                    } else {
                        cell_bg
                    };
                    tcell.set_style(Style::default().fg(glyph).bg(pal.accent));
                }
            }
        }
    }

    let metrics = pane.scroll_metrics();
    if metrics.max_offset_from_bottom > 0 && inner.width > 1 {
        let p = Palette::dark();
        let track = Rect::new(inner.right() - 1, content_area.top() + bh, 1, pty_h.saturating_sub(bh));
        crate::ui::widget::render_scrollbar(
            frame,
            metrics,
            track,
            p.surface1,
            p.overlay1,
            p.accent,
            &pane.state.prompt_anchors,
            "▐",
        );

        if metrics.offset_from_bottom == 0 {
            let buf = frame.buffer_mut();
            let gx = inner.right() - 1;
            let gy = content_area.top() + bh;
            if gx < buf.area().right() && gy < buf.area().bottom() {
                buf[(gx, gy)]
                    .set_symbol("↑")
                    .set_style(Style::default().fg(p.accent));
            }
        }
    }
}

#[allow(dead_code)]
pub fn pane_line<'a>(text: &'a str, style: Style) -> Line<'a> {
    Line::from(ratatui::text::Span::styled(text.to_string(), style))
}

