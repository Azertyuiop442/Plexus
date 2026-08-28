
use alacritty_terminal::grid::Dimensions;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use nf_icons::nf;

use crate::theme::Palette;
use crate::ui::pane::{BootInfo, MuxPane};

pub const BOX_H_LARGE: u16 = 12;
pub const BOX_H_SMALL: u16 = 8;

pub const COMMAND_ASCII_LOGO: [&str; 5] = [
    "███████  ███████  ███████████  ███████████  ███████  ████████  ███████ ",
    "███ ███  ██  ███  ███ ███ ███  ███ ███ ███  ███ ███  ███  ███  ███  ███",
    "███      ██  ███  ███ ███ ███  ███ ███ ███  ███████  ███  ███  ███  ███",
    "███ ███  ██  ███  ███ ███ ███  ███ ███ ███  ███████  ███  ███  ███  ███",
    "███████  ███████  ███ ███ ███  ███ ███ ███  ███ ███  ███  ███  ███████ ",
];

pub const CMD_ASCII_LOGO: [&str; 5] = [
    "███████  ███████████  ███████ ",
    "███ ███  ███ ███ ███  ███  ███",
    "███      ███ ███ ███  ███  ███",
    "███ ███  ███ ███ ███  ███  ███",
    "███████  ███ ███ ███  ███████ ",
];

const MIN_BANNER_W: u16 = 20;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BannerCardCache {
    pub key: (u16, u16, String, String, bool, Option<String>),
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else if max_len > 1 {
        let truncated: String = s.chars().take(max_len - 1).collect();
        format!("{}…", truncated)
    } else {
        "…".to_string()
    }
}

pub fn short_model_display(id: &str) -> String {
    match id.split_once('/') {
        Some((_vendor, base)) if !base.is_empty() => base.to_string(),
        _ => id.to_string(),
    }
}

pub fn ensure_boot_info(pane: &mut MuxPane, area: Rect) -> bool {
    if pane.state.boot_info.is_some() {
        return true;
    }

    let offset = pane.term.grid().display_offset();
    let scan_h = 24usize.min(pane.term.screen_lines());
    let scan_w = 160usize.min(pane.term.columns());
    let mut grid: Vec<Vec<char>> = vec![vec![' '; scan_w]; scan_h];
    let content = pane.term.renderable_content();
    for item in content.display_iter {
        if let Some(vp) = alacritty_terminal::term::point_to_viewport(offset, item.point) {
            let (x, y) = (vp.column.0 as usize, vp.line as usize);
            if y < scan_h && x < scan_w {
                grid[y][x] = item.cell.c;
            }
        }
    }
    let lines: Vec<String> = grid
        .into_iter()
        .map(|row| row.into_iter().collect())
        .collect();

    let version_pos = lines.iter().position(|l| {
        l.contains("Command Code v")
            || l.contains("cmd v")
            || l.contains("cmd  v")
            || l.contains("Command Code")
    });

    let (version_idx, version) = if let Some(idx) = version_pos {
        let l = &lines[idx];
        let v = if let Some(part) = l.split("Command Code v").nth(1) {
            part.split_whitespace().next().unwrap_or("").trim().to_string()
        } else if let Some(part) = l.split("cmd v").nth(1) {
            part.split_whitespace().next().unwrap_or("").trim().to_string()
        } else if let Some(part) = l.split("cmd  v").nth(1) {
            part.split_whitespace().next().unwrap_or("").trim().to_string()
        } else {
            String::new()
        };
        (idx, v)
    } else {
        (0, String::new())
    };

    let models = lines.iter().enumerate().find_map(|(idx, l)| {
        let trimmed = l.trim();
        if let Some(rest) = trimmed.strip_prefix("# models:") {
            let m = rest.trim();
            if !m.is_empty() {
                return Some(m.to_string());
            }
        }
        if let Some(rest) = trimmed.strip_prefix("models:") {
            let m = rest.trim();
            if !m.is_empty() {
                return Some(m.to_string());
            } else if idx + 1 < lines.len() {
                let next_line = lines[idx + 1].trim();
                if !next_line.is_empty()
                    && !next_line.starts_with("with")
                    && !next_line.starts_with('~')
                    && !next_line.starts_with('/')
                {
                    return Some(next_line.to_string());
                }
            }
        }
        if let Some(rest) = trimmed.strip_prefix("# model:") {
            let m = rest.trim();
            if !m.is_empty() {
                return Some(m.to_string());
            }
        }
        if let Some(rest) = trimmed.strip_prefix("model:") {
            let m = rest.trim();
            if !m.is_empty() {
                return Some(m.to_string());
            }
        }
        None
    });

    let cfg_model = if models.is_none() {
        let home = std::env::var("HOME").unwrap_or_default();
        let cfg_path = std::path::PathBuf::from(&home).join(".commandcode/config.json");
        std::fs::read_to_string(&cfg_path).ok().and_then(|raw| {
            serde_json::from_str::<serde_json::Value>(&raw).ok().and_then(|v| {
                v.get("model").and_then(|m| m.as_str()).map(|s| s.to_string())
            })
        })
    } else {
        None
    };
    let models = models.or(cfg_model);

    let parsed_cwd = lines.iter().find_map(|l| {
        let trimmed = l.trim();
        let stripped = trimmed.strip_prefix("# ").unwrap_or(trimmed);
        if (stripped.starts_with("~/") || stripped.starts_with('/'))
            && !stripped.contains("commandcode")
            && !stripped.contains("❯")
            && !stripped.contains("Ask")
        {
            let clean = stripped.trim_end_matches('?').trim();
            if !clean.is_empty() {
                return Some(clean.to_string());
            }
        }
        None
    });

    let home = std::env::var("HOME").unwrap_or_default();
    let short = |s: String| -> String {
        if !home.is_empty() && s.starts_with(&home) {
            format!("~{}", &s[home.len()..])
        } else {
            s
        }
    };
    let pending_cwd = pane
        .state
        .pending_cwd
        .take()
        .map(short)
        .filter(|c| c != "~");
    let real_cwd = std::env::current_dir()
        .ok()
        .map(|p| short(p.to_string_lossy().to_string()));

    let cwd = pending_cwd
        .or_else(|| parsed_cwd.filter(|c| c != "~"))
        .or(real_cwd);

    let mut native_logo: Vec<String> = if version_idx > 0 {
        lines[..version_idx]
            .iter()
            .map(|l| l.trim_end().to_string())
            .filter(|l| {
                !l.trim().is_empty()
                    && !l.contains("~/.commandcode")
                    && !l.contains("❯ cmd")
            })
            .collect()
    } else {
        Vec::new()
    };

    let is_real_wide_logo = native_logo.iter().any(|l| l.chars().count() >= 40);
    if !is_real_wide_logo {
        native_logo = COMMAND_ASCII_LOGO.iter().map(|s| s.to_string()).collect();
    }

    let version = if version.is_empty() {
        detect_version_via_binary()
    } else {
        version
    };

    pane.state.boot_info = Some(BootInfo {
        version,
        models,
        cwd,
        native_logo,
        capture_w: area.width,
    });

    true
}

fn detect_version_via_binary() -> String {
    for bin in ["commandcode", "cmdc", "cmd"] {
        if let Ok(out) = std::process::Command::new(bin).arg("--version").output() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() {
                return s;
            }
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            if !err.is_empty() {
                return err;
            }
        }
    }
    String::new()
}

pub fn is_banner_enabled() -> bool {
    let home = std::env::var("HOME").unwrap_or_default();
    if home.is_empty() {
        return true;
    }
    let cfg_path = std::path::PathBuf::from(&home).join(".commandcode/config.json");
    if let Ok(raw) = std::fs::read_to_string(&cfg_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) {
            if let Some(enabled) = json
                .get("show_banner")
                .or_else(|| json.get("show_ascii"))
                .or_else(|| json.get("welcome_banner"))
                .or_else(|| json.get("banner"))
                .or_else(|| json.get("show_welcome_banner"))
                .and_then(|v| v.as_bool())
            {
                return enabled;
            }
        }
    }
    true
}


fn is_terminal_in_interactive_menu(pane: &MuxPane) -> bool {
    let content = pane.term.renderable_content();
    if content.mode.contains(alacritty_terminal::term::TermMode::ALT_SCREEN) {
        return true;
    }

    let offset = pane.term.grid().display_offset();
    let scan_h = 35usize.min(pane.term.screen_lines());
    let scan_w = 120usize.min(pane.term.columns());
    let mut grid: Vec<Vec<char>> = vec![vec![' '; scan_w]; scan_h];
    for item in content.display_iter {
        if let Some(vp) = alacritty_terminal::term::point_to_viewport(offset, item.point) {
            let (x, y) = (vp.column.0 as usize, vp.line as usize);
            if y < scan_h && x < scan_w {
                grid[y][x] = item.cell.c;
            }
        }
    }
    let lines: Vec<String> = grid
        .into_iter()
        .map(|row| row.into_iter().collect::<String>())
        .collect();

    let full_text = lines.join("\n").to_lowercase();
    if full_text.contains("(use arrow keys)")
        || full_text.contains("(use \u{2191}\u{2193}")
        || full_text.contains("(press <space>")
        || full_text.contains("(press enter")
        || full_text.contains("❯ ◉")
        || full_text.contains("❯ ◯")
        || full_text.contains("❯ [ ]")
        || full_text.contains("❯ [x]")
    {
        return true;
    }

    for l in &lines {
        let t = l.trim().to_lowercase();
        if t.starts_with('?') && !t.contains("for shortcuts") {
            let after_q = t.trim_start_matches('?').trim_start();
            if after_q.starts_with("select ")
                || after_q.starts_with("choose ")
                || after_q.starts_with("configure ")
            {
                return true;
            }
        }
    }

    false
}

pub fn banner_height(pane: &mut MuxPane, area: Rect) -> u16 {
    if !is_banner_enabled()
        || area.width < MIN_BANNER_W
        || area.height < 6
        || is_terminal_in_interactive_menu(pane)
    {
        return 0;
    }

    let is_large = area.width >= 90 && area.height >= 12;
    let target_h = if is_large { BOX_H_LARGE } else { BOX_H_SMALL };
    let actual_h = target_h.min(area.height);

    let _ = ensure_boot_info(pane, area);

    actual_h
}

pub fn maybe_render(
    frame: &mut ratatui::Frame,
    area: Rect,
    pane: &mut MuxPane,
    recent: Option<&str>,
    yolo_mode: bool,
    active_model: Option<&str>,
    active_effort: Option<&str>,
) {
    let is_large = area.width >= 90 && area.height >= 10;
    let target_h = if is_large { BOX_H_LARGE } else { BOX_H_SMALL };
    let box_h = area.height.min(target_h);
    if !is_banner_enabled()
        || area.width < MIN_BANNER_W
        || box_h < 5
        || is_terminal_in_interactive_menu(pane)
    {
        return;
    }

    let _ = ensure_boot_info(pane, area);
    let Some(boot_info) = pane.state.boot_info.clone() else {
        return;
    };

    if is_large {
        render_large_banner(
            frame,
            area,
            box_h,
            pane,
            &boot_info,
            recent,
            yolo_mode,
            active_model,
            active_effort,
        );
    } else {
        render_compact_banner(
            frame,
            area,
            box_h,
            pane,
            &boot_info,
            yolo_mode,
            active_model,
            active_effort,
        );
    }
}

fn render_compact_banner(
    frame: &mut ratatui::Frame,
    area: Rect,
    box_h: u16,
    pane: &mut MuxPane,
    boot_info: &BootInfo,
    yolo_mode: bool,
    active_model: Option<&str>,
    active_effort: Option<&str>,
) {
    let p = Palette::dark();
    let bg = crate::theme::effective_bg();
    let accent = p.blue;
    let f_area = frame.area();

    let box_w = area.width;
    let x0 = area.x;
    let y0 = area.y;

    let min_inner_y = y0;
    let max_inner_y = y0.saturating_add(box_h).saturating_sub(2);

    let safe_cell = |frame: &mut ratatui::Frame, x: u16, y: u16, ch: &str, style: Style| {
        if x >= f_area.x
            && x < f_area.x + f_area.width
            && y >= f_area.y
            && y < f_area.y + f_area.height
        {
            frame.buffer_mut()[(x, y)].set_symbol(ch).set_style(style);
        }
    };

    for y in y0..y0 + box_h {
        for x in x0..x0 + box_w {
            safe_cell(frame, x, y, " ", Style::default().bg(bg));
        }
    }

    let border_style = Style::default().fg(accent);
    let div_y = y0 + box_h - 1;
    for x in x0..x0 + box_w {
        safe_cell(frame, x, div_y, "─", border_style);
    }
    safe_cell(frame, x0.saturating_sub(1), div_y, "├", border_style);
    safe_cell(frame, x0 + box_w, div_y, "┤", border_style);

    let title = if box_w < 45 {
        if boot_info.version.is_empty() {
            " cmd ".to_string()
        } else {
            format!(" cmd v{} ", boot_info.version)
        }
    } else {
        if boot_info.version.is_empty() {
            " Command Code ".to_string()
        } else {
            format!(" Command Code v{} ", boot_info.version)
        }
    };
    let top_border_y = y0.saturating_sub(1);
    for (i, ch) in title.chars().enumerate() {
        let x = x0 + 1 + i as u16;
        if x < x0 + box_w - 1 {
            safe_cell(
                frame,
                x,
                top_border_y,
                &ch.to_string(),
                Style::default()
                    .fg(accent)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            );
        }
    }

    let safe_put = |frame: &mut ratatui::Frame,
                    x: u16,
                    y: u16,
                    text: &str,
                    style: Style,
                    clamp_min_x: u16,
                    clamp_max_x: u16| {
        for (i, ch) in text.chars().enumerate() {
            let cx = x + i as u16;
            if cx >= clamp_min_x && cx <= clamp_max_x && y >= min_inner_y && y <= max_inner_y {
                safe_cell(frame, cx, y, &ch.to_string(), style);
            }
        }
    };

    let inner_min_x = x0;
    let inner_max_x = x0 + box_w - 1;
    let inner_cx = inner_min_x + (inner_max_x - inner_min_x) / 2;
    let max_text_len = ((inner_max_x - inner_min_x + 1) as usize).saturating_sub(1);

    let safe_put_spans = |frame: &mut ratatui::Frame,
                          y: u16,
                          spans: &[ratatui::text::Span],
                          clamp_min_x: u16,
                          clamp_max_x: u16| {
        let total_len: usize = spans.iter().map(|s| s.content.chars().count()).sum();
        let start_x = inner_cx.saturating_sub(total_len as u16 / 2);
        let mut curr_x = start_x;
        for span in spans {
            for ch in span.content.chars() {
                if curr_x >= clamp_min_x
                    && curr_x <= clamp_max_x
                    && y >= min_inner_y
                    && y <= max_inner_y
                {
                    safe_cell(frame, curr_x, y, &ch.to_string(), span.style);
                }
                curr_x += 1;
            }
        }
    };

    let logo: &[&str] = if box_w >= 76 {
        &COMMAND_ASCII_LOGO
    } else {
        &CMD_ASCII_LOGO
    };

    for (i, row) in logo.iter().enumerate() {
        let ly = y0 + 1 + i as u16;
        if ly <= max_inner_y {
            let len = row.chars().count() as u16;
            let start_x = inner_cx.saturating_sub(len / 2);
            safe_put(
                frame,
                start_x,
                ly,
                row,
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
                inner_min_x,
                inner_max_x,
            );
        }
    }

    let models_y = y0 + 1 + logo.len() as u16;
    let valid_model = active_model
        .filter(|m| !m.trim().is_empty() && !m.trim().eq_ignore_ascii_case("unknown"))
        .or_else(|| {
            boot_info
                .models
                .as_deref()
                .filter(|m| !m.trim().is_empty() && !m.trim().eq_ignore_ascii_case("unknown"))
        });
    if let Some(m_name) = valid_model {
        if models_y <= max_inner_y {
            let mut spans = vec![
                ratatui::text::Span::styled("models: ", Style::default().fg(p.subtext0)),
                ratatui::text::Span::styled(
                    short_model_display(m_name),
                    Style::default().fg(p.blue).add_modifier(Modifier::BOLD),
                ),
            ];

            if let Some(eff) = active_effort {
                let eff_color = match eff {
                    "Max" => p.red,
                    "X-High" => p.mauve,
                    "High" => p.peach,
                    "Medium" => p.yellow,
                    "Low" => p.green,
                    _ => p.subtext0,
                };
                spans.push(ratatui::text::Span::styled(
                    "  ·  ",
                    Style::default().fg(p.overlay0),
                ));
                spans.push(ratatui::text::Span::styled(
                    "effort: ",
                    Style::default().fg(p.subtext0),
                ));
                spans.push(ratatui::text::Span::styled(
                    eff.to_string(),
                    Style::default().fg(eff_color).add_modifier(Modifier::BOLD),
                ));
            }

            safe_put_spans(frame, models_y, &spans, inner_min_x, inner_max_x);
        }
    }

    let cwd_y = models_y + 1;
    if let Some(ref cwd) = boot_info.cwd {
        if cwd_y <= max_inner_y {
            let trunc = truncate_str(cwd, max_text_len.saturating_sub(3));
            let len = trunc.chars().count() as u16;
            let start_x = inner_cx.saturating_sub(len / 2);
            safe_put(
                frame,
                start_x,
                cwd_y,
                &trunc,
                Style::default().fg(p.subtext0),
                inner_min_x,
                inner_max_x,
            );
            let icon_x = start_x + len + 1;
            if icon_x <= inner_max_x {
                safe_cell(
                    frame,
                    icon_x,
                    cwd_y,
                    nf!("nf-cod-folder_opened"),
                    Style::default().fg(accent).add_modifier(Modifier::BOLD),
                );
                pane.state.banner_folder_icon = Some((icon_x, cwd_y));
            }
        }
    }

    if yolo_mode && cwd_y + 1 <= max_inner_y {
        let label = format!("{} YOLO mode on", nf!("nf-oct-zap"));
        let len = crate::ui::text::width(&label) as u16;
        let start_x = inner_cx.saturating_sub(len / 2);
        safe_put(
            frame,
            start_x,
            cwd_y + 1,
            &label,
            Style::default().fg(p.green).add_modifier(Modifier::BOLD),
            inner_min_x,
            inner_max_x,
        );
    }
}

fn render_large_banner(
    frame: &mut ratatui::Frame,
    area: Rect,
    box_h: u16,
    pane: &mut MuxPane,
    boot_info: &BootInfo,
    recent: Option<&str>,
    yolo_mode: bool,
    active_model: Option<&str>,
    active_effort: Option<&str>,
) {
    let p = Palette::dark();
    let bg = crate::theme::effective_bg();
    let accent = p.blue;
    let f_area = frame.area();

    let box_w = area.width;
    let x0 = area.x;
    let y0 = area.y;

    let min_inner_y = y0;
    let max_inner_y = y0.saturating_add(box_h).saturating_sub(2);

    let safe_cell = |frame: &mut ratatui::Frame, x: u16, y: u16, ch: &str, style: Style| {
        if x >= f_area.x
            && x < f_area.x + f_area.width
            && y >= f_area.y
            && y < f_area.y + f_area.height
        {
            frame.buffer_mut()[(x, y)].set_symbol(ch).set_style(style);
        }
    };

    for y in y0..y0 + box_h {
        for x in x0..x0 + box_w {
            safe_cell(frame, x, y, " ", Style::default().bg(bg));
        }
    }

    let border_style = Style::default().fg(accent);
    let div_y = y0 + box_h - 1;
    for x in x0..x0 + box_w {
        safe_cell(frame, x, div_y, "─", border_style);
    }
    safe_cell(frame, x0.saturating_sub(1), div_y, "├", border_style);
    safe_cell(frame, x0 + box_w, div_y, "┤", border_style);

    let title = if boot_info.version.is_empty() {
        " Command Code ".to_string()
    } else {
        format!(" Command Code v{} ", boot_info.version)
    };
    let top_border_y = y0.saturating_sub(1);
    for (i, ch) in title.chars().enumerate() {
        let x = x0 + 1 + i as u16;
        if x < x0 + box_w - 1 {
            safe_cell(
                frame,
                x,
                top_border_y,
                &ch.to_string(),
                Style::default()
                    .fg(accent)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            );
        }
    }

    let safe_put = |frame: &mut ratatui::Frame,
                    x: u16,
                    y: u16,
                    text: &str,
                    style: Style,
                    clamp_min_x: u16,
                    clamp_max_x: u16| {
        for (i, ch) in text.chars().enumerate() {
            let cx = x + i as u16;
            if cx >= clamp_min_x && cx <= clamp_max_x && y >= min_inner_y && y <= max_inner_y {
                safe_cell(frame, cx, y, &ch.to_string(), style);
            }
        }
    };

    let max_inner_x = x0 + box_w - 1;
    let max_logo_len = boot_info
        .native_logo
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(40) as u16;

    let left_width = (max_logo_len + 4)
        .max((box_w * 55) / 100)
        .min(box_w.saturating_sub(35));
    let left_min_x = x0 + 1;
    let left_max_x = (x0 + left_width).min(max_inner_x);
    let left_cx = left_min_x + (left_max_x - left_min_x) / 2;

    let right_min_x = left_max_x + 1;
    let right_max_x = max_inner_x;
    let right_width = right_max_x.saturating_sub(right_min_x) + 1;
    let right_cx = right_min_x + right_width / 2;

    let max_left_text_len = ((left_max_x - left_min_x + 1) as usize).saturating_sub(1);
    let logo: &[&str] = if max_left_text_len >= 72 {
        &COMMAND_ASCII_LOGO
    } else {
        &CMD_ASCII_LOGO
    };

    for (i, row) in logo.iter().enumerate() {
        let line_y = y0 + 2 + i as u16;
        if line_y <= max_inner_y {
            let trunc = truncate_str(row, max_left_text_len);
            let len = trunc.chars().count() as u16;
            let start_x = left_cx.saturating_sub(len / 2);
            safe_put(
                frame,
                start_x,
                line_y,
                &trunc,
                Style::default().fg(accent),
                left_min_x,
                left_max_x,
            );
        }
    }

    let safe_put_spans = |frame: &mut ratatui::Frame,
                          y: u16,
                          spans: &[ratatui::text::Span],
                          clamp_min_x: u16,
                          clamp_max_x: u16| {
        let total_len: usize = spans.iter().map(|s| s.content.chars().count()).sum();
        let start_x = left_cx.saturating_sub(total_len as u16 / 2);
        let mut curr_x = start_x;
        for span in spans {
            for ch in span.content.chars() {
                if curr_x >= clamp_min_x
                    && curr_x <= clamp_max_x
                    && y >= min_inner_y
                    && y <= max_inner_y
                {
                    safe_cell(frame, curr_x, y, &ch.to_string(), span.style);
                }
                curr_x += 1;
            }
        }
    };

    let next_left_y = y0 + 2 + logo.len() as u16 + 1;
    let valid_model = active_model
        .filter(|m| !m.trim().is_empty() && !m.trim().eq_ignore_ascii_case("unknown"))
        .or_else(|| {
            boot_info
                .models
                .as_deref()
                .filter(|m| !m.trim().is_empty() && !m.trim().eq_ignore_ascii_case("unknown"))
        });
    if let Some(m_name) = valid_model {
        if next_left_y <= max_inner_y {
            let mut spans = vec![
                ratatui::text::Span::styled("models: ", Style::default().fg(p.subtext0)),
                ratatui::text::Span::styled(
                    short_model_display(m_name),
                    Style::default().fg(p.blue).add_modifier(Modifier::BOLD),
                ),
            ];

            if let Some(eff) = active_effort {
                let eff_color = match eff {
                    "Max" => p.red,
                    "X-High" => p.mauve,
                    "High" => p.peach,
                    "Medium" => p.yellow,
                    "Low" => p.green,
                    _ => p.subtext0,
                };
                spans.push(ratatui::text::Span::styled(
                    "  ·  ",
                    Style::default().fg(p.overlay0),
                ));
                spans.push(ratatui::text::Span::styled(
                    "effort: ",
                    Style::default().fg(p.subtext0),
                ));
                spans.push(ratatui::text::Span::styled(
                    eff.to_string(),
                    Style::default().fg(eff_color).add_modifier(Modifier::BOLD),
                ));
            }

            safe_put_spans(frame, next_left_y, &spans, left_min_x, left_max_x);
        }
    }
    if let Some(ref cwd) = boot_info.cwd {
        if next_left_y + 1 <= max_inner_y {
            let trunc = truncate_str(cwd, max_left_text_len);
            let len = trunc.chars().count() as u16;
            let start_x = left_cx.saturating_sub(len / 2);
            safe_put(
                frame,
                start_x,
                next_left_y + 1,
                &trunc,
                Style::default().fg(p.subtext0),
                left_min_x,
                left_max_x,
            );
            let icon_x = start_x + len + 1;
            if icon_x <= left_max_x {
                safe_cell(
                    frame,
                    icon_x,
                    next_left_y + 1,
                    nf!("nf-cod-folder_opened"),
                    Style::default().fg(accent).add_modifier(Modifier::BOLD),
                );
                pane.state.banner_folder_icon = Some((icon_x, next_left_y + 1));
            }
        }
    }

    if yolo_mode && next_left_y + 2 <= max_inner_y {
        let label = format!("{} YOLO mode on", nf!("nf-oct-zap"));
        let len = crate::ui::text::width(&label) as u16;
        let start_x = left_cx.saturating_sub(len / 2);
        safe_put(
            frame,
            start_x,
            next_left_y + 2,
            &label,
            Style::default().fg(p.green).add_modifier(Modifier::BOLD),
            left_min_x,
            left_max_x,
        );
    }

    let put_cmd = |frame: &mut ratatui::Frame, cx: u16, y: u16, cmd: &str, desc: &str| {
        let total = cmd.chars().count() + 1 + desc.chars().count();
        let start = cx.saturating_sub(total as u16 / 2);
        safe_put(
            frame,
            start,
            y,
            cmd,
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
            right_min_x,
            right_max_x,
        );
        safe_put(
            frame,
            start + cmd.chars().count() as u16 + 1,
            y,
            desc,
            Style::default().fg(p.subtext0),
            right_min_x,
            right_max_x,
        );
    };

    let put_right_centered = |frame: &mut ratatui::Frame, y: u16, text: &str, style: Style| {
        let trunc = truncate_str(text, (right_width as usize).saturating_sub(2));
        let len = trunc.chars().count() as u16;
        let start_x = right_cx.saturating_sub(len / 2);
        safe_put(frame, start_x, y, &trunc, style, right_min_x, right_max_x);
    };

    put_right_centered(
        frame,
        y0 + 1,
        "Welcome back!",
        Style::default().fg(p.text).add_modifier(Modifier::BOLD),
    );
    put_cmd(frame, right_cx, y0 + 2, "Cmd+P", "keyboard shortcuts");
    put_cmd(frame, right_cx, y0 + 3, "/init", "scaffold AGENTS.md");
    put_cmd(frame, right_cx, y0 + 4, "/resume", "pick up past session");
    put_cmd(frame, right_cx, y0 + 5, "/model", "switch models");
    put_cmd(frame, right_cx, y0 + 6, "/help", "view documentation");

    if y0 + 7 <= max_inner_y {
        let rule_w = right_width.saturating_sub(4) as usize;
        let rule = "─".repeat(rule_w);
        put_right_centered(frame, y0 + 7, &rule, Style::default().fg(p.surface1));
    }

    put_right_centered(
        frame,
        y0 + 8,
        "Recent activity",
        Style::default().fg(accent),
    );
    match recent {
        Some(title) => {
            put_right_centered(frame, y0 + 9, title, Style::default().fg(p.subtext0))
        }
        None => put_right_centered(
            frame,
            y0 + 9,
            "No recent activity",
            Style::default().fg(p.subtext0),
        ),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yolo_label_rendered_in_both_compact_and_large() {
        let src = include_str!("banner.rs");
        let count = src.matches("YOLO mode on").count();
        assert!(
            count >= 2,
            "expected at least 2 'YOLO mode on' labels (compact + large), found {count}"
        );
    }

    #[test]
    fn yolo_label_does_not_use_red_or_panel_bg() {
        let src = include_str!("banner.rs");
        let start = src.find("YOLO mode on").unwrap();
        let window = &src[start.saturating_sub(120)..start + 20];
        assert!(
            !window.contains("p.red") && !window.contains("panel_bg"),
            "YOLO label still uses red or panel_bg: {window}"
        );
    }

    #[test]
    fn banner_stays_pinned_when_scrolled_into_scrollback() {
        use crate::ui::pane::MuxPane;
        use ratatui::layout::Rect;

        let Ok((pane, _reader)) = MuxPane::spawn("sleep 1", 100, 30) else {
            return;
        };
        let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());

        p.state.turns = 5;
        p.state.has_user_prompted = true;

        for i in 0..50 {
            p.feed(format!("old output line {i}\r\n").as_bytes());
        }

        let area = Rect::new(0, 0, 100, 30);
        let h_at_bottom = banner_height(&mut p, area);
        assert_eq!(h_at_bottom, BOX_H_LARGE);

        p.scroll_display(5);
        let h_scrolled = banner_height(&mut p, area);
        assert_eq!(h_scrolled, BOX_H_LARGE);

        p.scroll_reset();
        let h_after_reset = banner_height(&mut p, area);
        assert_eq!(h_after_reset, BOX_H_LARGE);
    }
}

