
#![allow(dead_code)]

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
    Frame,
};

use crate::theme::Palette;

pub fn truncate_line(line: &Line<'_>, max_width: usize) -> Line<'static> {
    let mut out = Vec::new();
    let mut used = 0usize;
    for span in line.spans.iter() {
        if used >= max_width {
            break;
        }
        let remain = max_width - used;
        let w = span.width();
        if w <= remain {
            out.push(Span::styled(span.content.to_string(), span.style));
            used += w;
            continue;
        }

        let mut clipped = String::new();
        let mut sw = 0usize;
        for ch in span.content.chars() {
            let cw = unicode_width::UnicodeWidthChar::width(ch)
                .unwrap_or(1)
                .max(1);
            if sw + cw > remain.saturating_sub(1) {
                break;
            }
            clipped.push(ch);
            sw += cw;
        }
        let mut tail = clipped;
        tail.push('…');
        out.push(Span::styled(tail, span.style));
        break;
    }
    Line::from(out)
}

pub fn panel_contrast_fg(p: &Palette) -> Color {
    p.text
}

pub fn centered_popup_rect(area: Rect, popup_w: u16, popup_h: u16) -> Option<Rect> {
    let popup_w = popup_w.min(area.width.saturating_sub(4));
    let popup_h = popup_h.min(area.height.saturating_sub(2));
    if popup_w < 4 || popup_h < 4 {
        return None;
    }
    let popup_x = area.x + (area.width.saturating_sub(popup_w)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_h)) / 2;
    Some(Rect::new(popup_x, popup_y, popup_w, popup_h))
}

pub struct Panel<'a> {
    pub border: Color,
    pub bg: Color,
    pub title: Option<&'a str>,
}

impl<'a> Panel<'a> {
    pub fn new(border: Color, bg: Color) -> Self {
        Self {
            border,
            bg,
            title: None,
        }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }
}

impl Widget for Panel<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        if area.width < 2 || area.height < 2 {
            return;
        }
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.border))
            .border_set(ratatui::symbols::border::ROUNDED)
            .title(self.title.unwrap_or(""))
            .title_style(
                Style::default()
                    .fg(self.border)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().bg(self.bg));
        Clear.render(area, buf);
        block.render(area, buf);
    }
}

pub struct Pill<'a> {
    pub text: &'a str,
    pub active: bool,
    pub accent: Color,
    pub fg_active: Color,
    pub fg_idle: Color,
    pub bg: Color,
}

impl<'a> Pill<'a> {
    pub fn new(text: &'a str, active: bool, p: &Palette) -> Self {
        Self {
            text,
            active,
            accent: p.accent,
            fg_active: panel_contrast_fg(p),
            fg_idle: p.overlay1,
            bg: p.panel_bg,
        }
    }

    pub fn style(&self) -> Style {
        if self.active {
            Style::default()
                .fg(self.fg_active)
                .bg(self.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.fg_idle).bg(self.bg)
        }
    }
}

impl Widget for Pill<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let style = self.style();
        let text = format!(" {} ", self.text);
        for (i, ch) in text.chars().enumerate() {
            let x = area.x + i as u16;
            if x < area.right() {
                buf[(x, area.y)].set_char(ch);
                buf[(x, area.y)].set_style(style);
            }
        }
    }
}

pub struct SelectableRow<'a> {
    pub spans: Vec<Span<'a>>,
    pub selected: bool,
    pub highlight_bg: Color,
    pub fg: Color,
    pub bg: Color,
}

impl<'a> SelectableRow<'a> {
    pub fn new(spans: Vec<Span<'a>>, selected: bool, p: &Palette) -> Self {
        Self {
            spans,
            selected,
            highlight_bg: p.surface0,
            fg: p.text,
            bg: p.sidebar_bg,
        }
    }

    pub fn styled(
        spans: Vec<Span<'a>>,
        selected: bool,
        bg: Color,
        highlight_bg: Color,
        fg: Color,
    ) -> Self {
        Self {
            spans,
            selected,
            highlight_bg,
            fg,
            bg,
        }
    }
}

impl Widget for SelectableRow<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let line_bg = if self.selected {
            self.highlight_bg
        } else {
            self.bg
        };
        for x in area.left()..area.right() {
            buf[(x, area.y)].set_bg(line_bg);
        }
        let mut x = area.left();
        for span in self.spans {
            let span_style = if self.selected {
                span.style.bg(self.highlight_bg).add_modifier(Modifier::BOLD)
            } else {
                span.style.bg(self.bg)
            };
            for ch in span.content.chars() {
                if x >= area.right() {
                    break;
                }
                buf[(x, area.y)].set_char(ch);
                buf[(x, area.y)].set_style(span_style);
                x += 1;
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollMetrics {
    pub max_offset_from_bottom: usize,
    pub offset_from_bottom: usize,
    pub viewport_rows: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollbarThumb {
    pub top: u16,
    pub len: u16,
}

pub fn scrollbar_thumb(metrics: ScrollMetrics, track: Rect) -> Option<ScrollbarThumb> {
    if metrics.max_offset_from_bottom == 0 || track.height == 0 {
        return None;
    }
    let track_height = track.height as usize;
    let total_rows = metrics.max_offset_from_bottom + metrics.viewport_rows;
    if total_rows == 0 {
        return None;
    }
    let thumb_len = ((metrics.viewport_rows * track_height) as f32 / total_rows as f32)
        .round()
        .max(1.0)
        .min(track_height as f32) as usize;
    let max_thumb_top = track_height.saturating_sub(thumb_len);
    let scrolled_from_top = metrics
        .max_offset_from_bottom
        .saturating_sub(metrics.offset_from_bottom);
    let thumb_top = if max_thumb_top == 0 || metrics.max_offset_from_bottom == 0 {
        0
    } else {
        ((scrolled_from_top * max_thumb_top) as f32 / metrics.max_offset_from_bottom as f32)
            .round()
            .clamp(0.0, max_thumb_top as f32) as usize
    };
    Some(ScrollbarThumb {
        top: track.y + thumb_top as u16,
        len: thumb_len as u16,
    })
}

pub fn render_scrollbar(
    frame: &mut Frame,
    metrics: ScrollMetrics,
    track: Rect,
    track_color: Color,
    thumb_color: Color,
    prompt_anchor_color: Color,
    prompt_anchors: &[i32],
    _thumb_symbol: &str,
) {
    if metrics.max_offset_from_bottom == 0 || track.width == 0 || track.height == 0 {
        return;
    }
    let Some(thumb) = scrollbar_thumb(metrics, track) else {
        return;
    };
    let buf = frame.buffer_mut();
    let total_rows = metrics.max_offset_from_bottom + metrics.viewport_rows;
    let track_h = track.height as i32;

    for y in track.y..track.y + track.height {
        let cell = &mut buf[(track.x, y)];
        cell.set_symbol(" ");
        let in_thumb = y >= thumb.top && y < thumb.top + thumb.len;
        if in_thumb {
            cell.set_bg(thumb_color);
            continue;
        }

        let mut is_anchor = false;
        if total_rows > 0 {
            for &a in prompt_anchors {
                let offset_from_oldest = (a as i64 + metrics.max_offset_from_bottom as i64)
                    .clamp(0, total_rows as i64) as i64;
                let row = ((offset_from_oldest * track_h as i64) / total_rows as i64) as i32;
                let track_row = (track.y as i32) + row;
                if track_row == y as i32 {
                    is_anchor = true;
                    break;
                }
            }
        }
        cell.set_bg(if is_anchor { prompt_anchor_color } else { track_color });
    }
}

pub fn render_modal_shell(
    frame: &mut Frame,
    area: Rect,
    popup_w: u16,
    popup_h: u16,
    _p: &Palette,
) -> Option<Rect> {
    let popup = centered_popup_rect(area, popup_w, popup_h)?;
    let pitch_black = Color::Rgb(0, 0, 0);
    let border_blue = Palette::dark().blue;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_blue))
        .border_set(ratatui::symbols::border::ROUNDED)
        .style(Style::default().bg(pitch_black));
    let inner = block.inner(popup);
    Clear.render(popup, frame.buffer_mut());
    block.render(popup, frame.buffer_mut());
    Some(inner)
}

pub fn render_modal_header(frame: &mut Frame, area: Rect, title: &str, p: &Palette) {
    let line = Line::from(vec![Span::styled(
        title,
        Style::default().fg(p.text).add_modifier(Modifier::BOLD),
    )]);
    frame.render_widget(Paragraph::new(line), area);
}

pub fn action_button_text(hint: Option<&str>, label: &str) -> String {
    match hint {
        Some(hint) => format!(" {hint} {label} "),
        None => format!(" {label} "),
    }
}

pub fn action_button_width(hint: Option<&str>, label: &str) -> u16 {
    action_button_text(hint, label).chars().count() as u16
}

pub struct ActionButtonSpec<'a> {
    pub hint: Option<&'a str>,
    pub label: &'a str,
}

pub fn centered_button_row(inner: Rect, widths: &[u16], gap: u16, row_offset: u16) -> Vec<Rect> {
    let total_w = widths
        .iter()
        .copied()
        .sum::<u16>()
        .saturating_add(gap.saturating_mul(widths.len().saturating_sub(1) as u16));
    let mut x = inner.x + inner.width.saturating_sub(total_w) / 2;
    let y = inner.y + row_offset.min(inner.height.saturating_sub(1));
    widths
        .iter()
        .map(|w| {
            let rect = Rect::new(
                x,
                y,
                (*w).min(inner.width.saturating_sub(x.saturating_sub(inner.x))),
                1,
            );
            x = x.saturating_add(*w).saturating_add(gap);
            rect
        })
        .collect()
}

pub fn action_button_row_rects(
    area: Rect,
    buttons: &[ActionButtonSpec<'_>],
    gap: u16,
    row_offset: u16,
) -> Vec<Rect> {
    let widths: Vec<u16> = buttons
        .iter()
        .map(|b| action_button_width(b.hint, b.label))
        .collect();
    centered_button_row(area, &widths, gap, row_offset)
}

#[allow(unused_imports)]
pub use crate::ui::widgets::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pill_active_uses_accent_bg_and_contrast_fg() {
        let p = Palette::dark();
        let active = Pill::new("terminal 1", true, &p).style();
        assert_eq!(active.bg, Some(p.accent));
        assert_eq!(active.fg, Some(panel_contrast_fg(&p)));
        assert!(active.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn pill_idle_uses_overlay1_on_panel_bg() {
        let p = Palette::dark();
        let idle = Pill::new("terminal 2", false, &p).style();
        assert_eq!(idle.bg, Some(p.panel_bg));
        assert_eq!(idle.fg, Some(p.overlay1));
        assert!(!idle.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn scrollbar_anchors_render_at_expected_track_rows() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(1, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let track = frame.area();
                let metrics = ScrollMetrics {
                    max_offset_from_bottom: 50,
                    offset_from_bottom: 0,
                    viewport_rows: 10,
                };
                render_scrollbar(
                    frame,
                    metrics,
                    track,
                    Color::Rgb(1, 1, 1),
                    Color::Rgb(2, 2, 2),
                    Color::Rgb(3, 3, 3),
                    &[-50, -25, 0],
                    "▐",
                );
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        let cells: Vec<_> = (0..10)
            .map(|y| {
                let c = &buf[(0, y)];
                match c.bg {
                    Color::Rgb(r, g, b) => (r, g, b),
                    other => panic!("row {y}: expected RGB, got {other:?}"),
                }
            })
            .collect();

        assert_eq!(cells[0], (3, 3, 3), "row 0 should be the -50 anchor (accent)");

        assert_eq!(cells[4], (3, 3, 3), "row 4 should be the -25 anchor (accent)");

        for &y in &[1usize, 2, 3, 6, 7] {
            assert_eq!(cells[y], (1, 1, 1), "row {y} should be track (not thumb/anchor)");
        }

        assert_eq!(cells[8], (2, 2, 2), "row 8 should be thumb");
        assert_eq!(cells[9], (2, 2, 2), "row 9 should be thumb");
    }

    #[test]
    fn scrollbar_no_anchors_renders_only_thumb_and_track() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(1, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let track = frame.area();
                let metrics = ScrollMetrics {
                    max_offset_from_bottom: 50,
                    offset_from_bottom: 0,
                    viewport_rows: 10,
                };
                render_scrollbar(
                    frame,
                    metrics,
                    track,
                    Color::Rgb(1, 1, 1),
                    Color::Rgb(2, 2, 2),
                    Color::Rgb(3, 3, 3),
                    &[],
                    "▐",
                );
            })
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        for y in 0..10 {
            let c = &buf[(0, y)];
            assert!(
                matches!(c.bg, Color::Rgb(1, 1, 1) | Color::Rgb(2, 2, 2)),
                "row {y} must be track (1) or thumb (2), got {:?}",
                c.bg
            );
        }
    }

    #[test]
    fn scrollbar_thumb_sizes_and_positions() {
        let track = Rect::new(0, 0, 1, 10);

        assert_eq!(
            scrollbar_thumb(
                ScrollMetrics {
                    max_offset_from_bottom: 0,
                    offset_from_bottom: 0,
                    viewport_rows: 10
                },
                track,
            ),
            None
        );

        let thumb = scrollbar_thumb(
            ScrollMetrics {
                max_offset_from_bottom: 20,
                offset_from_bottom: 0,
                viewport_rows: 10,
            },
            track,
        )
        .unwrap();
        assert_eq!(thumb.top, 7);
        assert_eq!(thumb.len, 3);

        let thumb = scrollbar_thumb(
            ScrollMetrics {
                max_offset_from_bottom: 20,
                offset_from_bottom: 20,
                viewport_rows: 10,
            },
            track,
        )
        .unwrap();
        assert_eq!(thumb.top, 0);
        assert_eq!(thumb.len, 3);
    }

    #[test]
    fn centered_button_row_centers_buttons() {
        let area = Rect::new(0, 0, 20, 5);
        let widths = [5, 5];
        let rects = centered_button_row(area, &widths, 1, 3);

        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].x, 4);
        assert_eq!(rects[1].x, 10);
        assert_eq!(rects[0].y, 3);
    }

    #[test]
    fn panel_returns_usable_inner_area() {
        let area = Rect::new(0, 0, 10, 10);
        let inner = centered_popup_rect(area, 6, 6).unwrap();
        assert_eq!(inner.width, 6);
        assert_eq!(inner.height, 6);
        assert_eq!(inner.x, 2);
        assert_eq!(inner.y, 2);
    }
}

