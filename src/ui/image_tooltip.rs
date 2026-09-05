
use std::path::{Path, PathBuf};
use ratatui::buffer::Buffer;
use ratatui::layout::{Rect, Size};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget};
use ratatui::Frame;
use image::GenericImageView;
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::protocol::Protocol;
use ratatui_image::{Image as RatatuiImageWidget, Resize};

use crate::state::HoverImage;
use crate::theme::Palette;
use crate::ui::widget::render_drop_shadow;

pub fn cache_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("cc-image-cache");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn image_extension_for_bytes(bytes: &[u8]) -> &'static str {
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        "jpg"
    } else if bytes.len() >= 4 && bytes[0] == 0x89 && bytes[1] == b'P' && bytes[2] == b'N' && bytes[3] == b'G' {
        "png"
    } else if bytes.len() >= 4 && bytes[0] == b'R' && bytes[1] == b'I' && bytes[2] == b'F' && bytes[3] == b'F' {
        "webp"
    } else if bytes.len() >= 4 && bytes[0] == b'G' && bytes[1] == b'I' && bytes[2] == b'F' {
        "gif"
    } else {
        "png"
    }
}

pub fn resolve_image_file(index: usize, session_id: Option<&str>) -> Option<PathBuf> {
    let mut temp_dirs = vec![std::env::temp_dir().join("commandcode-images")];
    let alt_tmp = PathBuf::from("/tmp/commandcode-images");
    if !temp_dirs.contains(&alt_tmp) {
        temp_dirs.push(alt_tmp);
    }

    let prefix = format!("image-{index}-");
    let mut candidates: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    for tdir in temp_dirs {
        if let Ok(entries) = std::fs::read_dir(tdir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(&prefix) {
                    let mtime = entry.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
                    candidates.push((mtime, entry.path()));
                }
            }
        }
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    if let Some((_, path)) = candidates.into_iter().next() {
        return Some(path);
    }

    let dir = cache_dir();
    let base_dest = if let Some(sid) = session_id {
        dir.join(format!("img_{sid}_{index}"))
    } else {
        dir.join(format!("img_recent_{index}"))
    };

    if let Some(extracted) = extract_from_session_transcript(session_id, index, &base_dest) {
        return Some(extracted);
    }

    let clip_dest = dir.join(format!("clip_{index}.png"));
    if let Some(clip) = dump_os_clipboard(&clip_dest) {
        return Some(clip);
    }

    for ext in &["png", "jpg", "jpeg", "webp", "gif"] {
        let cached = if let Some(sid) = session_id {
            dir.join(format!("img_{sid}_{index}.{ext}"))
        } else {
            dir.join(format!("img_recent_{index}.{ext}"))
        };
        if cached.exists() && cached.metadata().map(|m| m.len() > 100).unwrap_or(false) {
            return Some(cached);
        }
    }

    None
}

fn find_images_recursive(val: &serde_json::Value, results: &mut Vec<String>) {
    match val {
        serde_json::Value::Object(map) => {
            if map.get("type").and_then(|v| v.as_str()) == Some("image") {
                if let Some(data) = map.get("source").and_then(|s| s.get("data")).and_then(|d| d.as_str()) {
                    results.push(data.to_string());
                } else if let Some(data) = map.get("data").and_then(|v| v.as_str()) {
                    results.push(data.to_string());
                }
            }
            for v in map.values() {
                find_images_recursive(v, results);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                find_images_recursive(v, results);
            }
        }
        _ => {}
    }
}

fn extract_image_from_file(path: &Path, target_idx: usize, base_dest: &Path) -> Option<PathBuf> {
    let content = std::fs::read_to_string(path).ok()?;
    let lines: Vec<&str> = content.lines().collect();
    for line in lines.into_iter().rev() {
        if !line.contains("\"type\":\"image\"") && !line.contains("\"type\": \"image\"") {
            continue;
        }
        let Ok(val) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let mut images = Vec::new();
        find_images_recursive(&val, &mut images);
        if !images.is_empty() && target_idx <= images.len() && target_idx > 0 {
            use base64::prelude::*;
            if let Ok(decoded) = BASE64_STANDARD.decode(&images[target_idx - 1]) {
                let ext = image_extension_for_bytes(&decoded);
                let dest = base_dest.with_extension(ext);
                if std::fs::write(&dest, decoded).is_ok() {
                    return Some(dest);
                }
            }
        }
    }
    None
}

fn extract_from_session_transcript(sid: Option<&str>, target_idx: usize, dest: &Path) -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let projects_dir = PathBuf::from(home).join(".commandcode/projects");
    if !projects_dir.exists() {
        return None;
    }

    if let Some(sid) = sid {
        let pattern = format!("{sid}.jsonl");
        if let Ok(entries) = std::fs::read_dir(&projects_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let candidate = p.join(&pattern);
                    if candidate.exists() {
                        if let Some(found) = extract_image_from_file(&candidate, target_idx, dest) {
                            return Some(found);
                        }
                    }
                }
            }
        }
    }

    let mut candidate_files: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&projects_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                if let Ok(sub_entries) = std::fs::read_dir(&p) {
                    for sub in sub_entries.flatten() {
                        let path = sub.path();
                        if path.extension().and_then(|e| e.to_str()) == Some("jsonl")
                            && !path.to_string_lossy().ends_with(".checkpoints.jsonl")
                        {
                            let mtime = path.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
                            candidate_files.push((mtime, path));
                        }
                    }
                }
            }
        }
    }

    candidate_files.sort_by(|a, b| b.0.cmp(&a.0));

    for (_, path) in candidate_files.iter().take(8) {
        if let Some(found) = extract_image_from_file(path, target_idx, dest) {
            return Some(found);
        }
    }

    None
}

fn dump_os_clipboard(dest: &Path) -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "try\nset has_img to false\ntry\nthe clipboard as «class PNGf»\nset has_img to true\non error\ntry\nthe clipboard as «class TIFF»\nset has_img to true\nend try\nend try\nif has_img then\nset f to open for access POSIX file \"{}\" with write permission\nset eof f to 0\ntry\nwrite (the clipboard as «class PNGf») to f\non error\nwrite (the clipboard as «class TIFF») to f\nend try\nclose access f\nend if\nend try",
            dest.display()
        );
        let status = std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .status()
            .ok()?;
        if status.success() && dest.exists() && dest.metadata().map(|m| m.len() > 100).unwrap_or(false) {
            return Some(dest.to_path_buf());
        } else if dest.exists() {
            let _ = std::fs::remove_file(dest);
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::process::Command::new("wl-paste")
            .args(["-t", "image/png"])
            .stdout(std::fs::File::create(dest).ok()?)
            .status()
        {
            if status.success() && dest.exists() && dest.metadata().map(|m| m.len() > 100).unwrap_or(false) {
                return Some(dest.to_path_buf());
            } else if dest.exists() {
                let _ = std::fs::remove_file(dest);
            }
        }
        if let Ok(status) = std::process::Command::new("xclip")
            .args(["-selection", "clipboard", "-t", "image/png", "-o"])
            .stdout(std::fs::File::create(dest).ok()?)
            .status()
        {
            if status.success() && dest.exists() && dest.metadata().map(|m| m.len() > 100).unwrap_or(false) {
                return Some(dest.to_path_buf());
            } else if dest.exists() {
                let _ = std::fs::remove_file(dest);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let ps_cmd = format!(
            "Add-Type -AssemblyName System.Windows.Forms; $img = [System.Windows.Forms.Clipboard]::GetImage(); if ($img) {{ $img.Save('{}', [System.Drawing.Imaging.ImageFormat]::Png) }}",
            dest.display()
        );
        let status = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_cmd])
            .status()
            .ok()?;
        if status.success() && dest.exists() && dest.metadata().map(|m| m.len() > 100).unwrap_or(false) {
            return Some(dest.to_path_buf());
        } else if dest.exists() {
            let _ = std::fs::remove_file(dest);
        }
    }

    None
}

#[derive(Clone)]
struct CachedProto {
    path: PathBuf,
    mtime: std::time::SystemTime,
    area_w: u16,
    area_h: u16,
    proto: Protocol,
    orig_w: u32,
    orig_h: u32,
}

static PROTO_CACHE: std::sync::Mutex<Vec<CachedProto>> = std::sync::Mutex::new(Vec::new());

fn get_cell_size_px() -> (u16, u16) {
    if let Ok(ws) = crossterm::terminal::window_size() {
        if ws.columns > 0 && ws.rows > 0 && ws.width > 0 && ws.height > 0 {
            let cw = (ws.width / ws.columns).max(1);
            let ch = (ws.height / ws.rows).max(1);
            return (cw, ch);
        }
    }
    #[cfg(unix)]
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) == 0 {
            if ws.ws_col > 0 && ws.ws_row > 0 && ws.ws_xpixel > 0 && ws.ws_ypixel > 0 {
                let cw = (ws.ws_xpixel / ws.ws_col).max(1);
                let ch = (ws.ws_ypixel / ws.ws_row).max(1);
                return (cw, ch);
            }
        }
    }
    let is_ghostty = std::env::var("TERM_PROGRAM").map(|v| v == "ghostty").unwrap_or(false)
        || std::env::var("GHOSTTY_BIN_DIR").is_ok()
        || std::env::var("GHOSTTY_RESOURCES_DIR").is_ok();
    if is_ghostty {
        (14, 32)
    } else {
        (10, 20)
    }
}

fn create_picker() -> Picker {
    let (cw, ch) = get_cell_size_px();
    #[allow(deprecated)]
    let mut picker = Picker::from_fontsize(ratatui_image::FontSize::new(cw, ch));
    let is_ghostty = std::env::var("TERM_PROGRAM").map(|v| v == "ghostty").unwrap_or(false)
        || std::env::var("GHOSTTY_BIN_DIR").is_ok()
        || std::env::var("GHOSTTY_RESOURCES_DIR").is_ok();
    let is_kitty = std::env::var("KITTY_WINDOW_ID").is_ok()
        || std::env::var("TERM").map(|v| v.contains("kitty")).unwrap_or(false);
    let is_iterm = std::env::var("TERM_PROGRAM").map(|v| v.contains("iTerm")).unwrap_or(false);
    let is_wezterm = std::env::var("WEZTERM_EXECUTABLE").is_ok() || std::env::var("WEZTERM_PANE").is_ok();

    if is_ghostty || is_kitty {
        picker.set_protocol_type(ProtocolType::Kitty);
    } else if is_iterm || is_wezterm {
        picker.set_protocol_type(ProtocolType::Iterm2);
    }
    picker
}

static PICKER: std::sync::OnceLock<Picker> = std::sync::OnceLock::new();

pub fn get_picker() -> &'static Picker {
    PICKER.get_or_init(create_picker)
}

#[derive(Clone)]
struct CachedThumbnail {
    path: PathBuf,
    cols: u32,
    rows: u32,
    pixels: Vec<(Color, Color)>,
    orig_w: u32,
    orig_h: u32,
}

static THUMB_CACHE: std::sync::Mutex<Vec<CachedThumbnail>> = std::sync::Mutex::new(Vec::new());

pub fn render_image_preview(buf: &mut Buffer, area: Rect, path: &Path) -> Option<(u32, u32)> {
    if area.width < 2 || area.height < 2 {
        return None;
    }

    let mtime = path.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);

    if let Ok(guard) = PROTO_CACHE.lock() {
        for cached in guard.iter().rev() {
            if cached.path == path && cached.mtime == mtime && cached.area_w == area.width && cached.area_h == area.height {
                let size = cached.proto.size();
                let fit_w = size.width.min(area.width);
                let fit_h = size.height.min(area.height);
                let pad_x = (area.width.saturating_sub(fit_w)) / 2;
                let pad_y = (area.height.saturating_sub(fit_h)) / 2;
                let target_rect = Rect::new(area.x + pad_x, area.y + pad_y, fit_w, fit_h);
                let widget = RatatuiImageWidget::new(&cached.proto).allow_clipping(true);
                widget.render(target_rect, buf);
                return Some((cached.orig_w, cached.orig_h));
            }
        }
    }

    let bytes = std::fs::read(path).ok()?;
    let img = image::load_from_memory(&bytes).ok()?;
    let (orig_w, orig_h) = img.dimensions();
    if orig_w == 0 || orig_h == 0 {
        return None;
    }

    let picker = get_picker();
    if let Ok(proto) = picker.new_protocol(img.clone(), Size::new(area.width, area.height), Resize::Fit(None)) {
        let size = proto.size();
        let fit_w = size.width.min(area.width);
        let fit_h = size.height.min(area.height);
        let pad_x = (area.width.saturating_sub(fit_w)) / 2;
        let pad_y = (area.height.saturating_sub(fit_h)) / 2;
        let target_rect = Rect::new(area.x + pad_x, area.y + pad_y, fit_w, fit_h);
        let widget = RatatuiImageWidget::new(&proto).allow_clipping(true);
        widget.render(target_rect, buf);

        if let Ok(mut guard) = PROTO_CACHE.lock() {
            if guard.len() >= 6 {
                guard.remove(0);
            }
            guard.push(CachedProto {
                path: path.to_path_buf(),
                mtime,
                area_w: area.width,
                area_h: area.height,
                proto,
                orig_w,
                orig_h,
            });
        }
        return Some((orig_w, orig_h));
    }

    let max_cols = area.width as u32;
    let max_pixel_rows = (area.height as u32) * 2;

    if let Ok(guard) = THUMB_CACHE.lock() {
        for cached in guard.iter().rev() {
            if cached.path == path {
                let scale_x = max_cols as f32 / cached.orig_w as f32;
                let scale_y = max_pixel_rows as f32 / cached.orig_h as f32;
                let scale = scale_x.min(scale_y);
                let needed_cols = ((cached.orig_w as f32 * scale).round() as u32).max(1).min(max_cols);
                let needed_rows = ((cached.orig_h as f32 * scale).round() as u32).max(2).min(max_pixel_rows);
                let needed_rows = if needed_rows % 2 != 0 { needed_rows + 1 } else { needed_rows };

                if cached.cols == needed_cols && cached.rows == needed_rows {
                    let pad_x = (area.width.saturating_sub(cached.cols as u16)) / 2;
                    let pad_y = (area.height.saturating_sub((cached.rows / 2) as u16)) / 2;
                    let mut pixel_idx = 0;
                    for cell_y in 0..(cached.rows / 2) {
                        let screen_y = area.y + pad_y + cell_y as u16;
                        if screen_y >= area.bottom() {
                            break;
                        }
                        for cell_x in 0..cached.cols {
                            let screen_x = area.x + pad_x + cell_x as u16;
                            if screen_x < area.right() && pixel_idx < cached.pixels.len() {
                                let (fg, bg) = cached.pixels[pixel_idx];
                                let cell = &mut buf[(screen_x, screen_y)];
                                cell.set_symbol("▀");
                                cell.set_style(Style::default().fg(fg).bg(bg));
                            }
                            pixel_idx += 1;
                        }
                    }
                    return Some((cached.orig_w, cached.orig_h));
                }
            }
        }
    }

    let scale_x = max_cols as f32 / orig_w as f32;
    let scale_y = max_pixel_rows as f32 / orig_h as f32;
    let scale = scale_x.min(scale_y);

    let target_cols = ((orig_w as f32 * scale).round() as u32).max(1).min(max_cols);
    let target_rows_px = ((orig_h as f32 * scale).round() as u32).max(2).min(max_pixel_rows);
    let target_rows_px = if target_rows_px % 2 != 0 { target_rows_px + 1 } else { target_rows_px };

    let resized = img.resize(target_cols, target_rows_px, image::imageops::FilterType::Lanczos3);
    let rgba = resized.to_rgba8();

    let pad_x = (area.width.saturating_sub(target_cols as u16)) / 2;
    let pad_y = (area.height.saturating_sub((target_rows_px / 2) as u16)) / 2;

    let mut cached_pixels = Vec::with_capacity((target_cols * (target_rows_px / 2)) as usize);
    let bg_r = 16.0f32;
    let bg_g = 15.0f32;
    let bg_b = 15.0f32;

    for cell_y in 0..(target_rows_px / 2) {
        let screen_y = area.y + pad_y + cell_y as u16;
        let top_px_y = cell_y * 2;
        let btm_px_y = cell_y * 2 + 1;

        for cell_x in 0..target_cols {
            let screen_x = area.x + pad_x + cell_x as u16;

            let top = rgba.get_pixel(cell_x, top_px_y);
            let btm = rgba.get_pixel(cell_x, btm_px_y);

            let top_a = top[3] as f32 / 255.0;
            let fg_r = ((top[0] as f32 * top_a) + (bg_r * (1.0 - top_a))).round() as u8;
            let fg_g = ((top[1] as f32 * top_a) + (bg_g * (1.0 - top_a))).round() as u8;
            let fg_b = ((top[2] as f32 * top_a) + (bg_b * (1.0 - top_a))).round() as u8;

            let btm_a = btm[3] as f32 / 255.0;
            let bg_pr = ((btm[0] as f32 * btm_a) + (bg_r * (1.0 - btm_a))).round() as u8;
            let bg_pg = ((btm[1] as f32 * btm_a) + (bg_g * (1.0 - btm_a))).round() as u8;
            let bg_pb = ((btm[2] as f32 * btm_a) + (bg_b * (1.0 - btm_a))).round() as u8;

            let fg = Color::Rgb(fg_r, fg_g, fg_b);
            let bg = Color::Rgb(bg_pr, bg_pg, bg_pb);
            cached_pixels.push((fg, bg));

            if screen_y < area.bottom() && screen_x < area.right() {
                let cell = &mut buf[(screen_x, screen_y)];
                cell.set_symbol("▀");
                cell.set_style(Style::default().fg(fg).bg(bg));
            }
        }
    }

    if let Ok(mut guard) = THUMB_CACHE.lock() {
        if guard.len() >= 8 {
            guard.remove(0);
        }
        guard.push(CachedThumbnail {
            path: path.to_path_buf(),
            cols: target_cols,
            rows: target_rows_px,
            pixels: cached_pixels,
            orig_w,
            orig_h,
        });
    }

    Some((orig_w, orig_h))
}

pub fn image_dimensions(path: &Path) -> Option<(u32, u32)> {
    if let Ok(guard) = PROTO_CACHE.lock() {
        for cached in guard.iter().rev() {
            if cached.path == path {
                return Some((cached.orig_w, cached.orig_h));
            }
        }
    }
    if let Ok(guard) = THUMB_CACHE.lock() {
        for cached in guard.iter().rev() {
            if cached.path == path {
                return Some((cached.orig_w, cached.orig_h));
            }
        }
    }
    if let Ok((w, h)) = image::image_dimensions(path) {
        return Some((w, h));
    }
    let bytes = std::fs::read(path).ok()?;
    let img = image::load_from_memory(&bytes).ok()?;
    Some(img.dimensions())
}

pub fn calculate_tooltip_rect(
    screen_x: u16,
    screen_y: u16,
    _index: usize,
    screen: Rect,
    _session_id: Option<&str>,
) -> Rect {
    const TOOLTIP_W: u16 = 50;
    const TOOLTIP_H: u16 = 14;

    let popup_w = TOOLTIP_W.min(screen.width.saturating_sub(4)).max(20);
    let popup_h = TOOLTIP_H.min(screen.height.saturating_sub(4)).max(8);

    let x = screen_x
        .saturating_sub(2)
        .min(screen.width.saturating_sub(popup_w));
    let y = if screen_y + 1 + popup_h <= screen.height {
        screen_y + 1
    } else if screen_y >= popup_h {
        screen_y.saturating_sub(popup_h)
    } else {
        screen.height.saturating_sub(popup_h) / 2
    };

    Rect::new(x, y, popup_w, popup_h)
}

#[allow(dead_code)]
pub fn tooltip_rect(hover: &HoverImage, screen: Rect, session_id: Option<&str>) -> Rect {
    calculate_tooltip_rect(hover.screen_x, hover.screen_y, hover.index, screen, session_id)
}

pub fn render_image_tooltip(
    frame: &mut Frame,
    hover: &HoverImage,
    screen: Rect,
    session_id: Option<&str>,
) {
    let Some(path) = resolve_image_file(hover.index, session_id) else {
        return;
    };
    let Some(dims) = image_dimensions(&path) else {
        return;
    };

    let popup = hover.popup_rect;
    if popup.width < 4 || popup.height < 4 {
        return;
    }

    let p = Palette::dark();
    let border_style = Style::default().fg(p.accent);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(ratatui::symbols::border::ROUNDED)
        .border_style(border_style)
        .style(Style::default().bg(p.sidebar_bg));

    let inner = block.inner(popup);
    if inner.height < 2 || inner.width < 4 {
        return;
    }

    let caption_h = 1u16;
    let preview_area = Rect::new(
        inner.x,
        inner.y,
        inner.width,
        inner.height.saturating_sub(caption_h),
    );
    let caption_area = Rect::new(
        inner.x,
        inner.bottom().saturating_sub(caption_h),
        inner.width,
        caption_h,
    );

    let buf = frame.buffer_mut();
    render_drop_shadow(buf, popup, screen);
    Clear.render(popup, buf);
    block.render(popup, buf);

    let header_text = format!(" Image #{} ", hover.index);
    let header_style = Style::default().fg(p.accent).bg(p.sidebar_bg).add_modifier(Modifier::BOLD);
    let title_x = popup.x + 2;
    for (i, ch) in header_text.chars().enumerate() {
        let cx = title_x + i as u16;
        if cx < popup.right() - 1 {
            buf[(cx, popup.y)]
                .set_symbol(&ch.to_string())
                .set_style(header_style);
        }
    }

    render_image_preview(buf, preview_area, &path);

    let (w, h) = dims;
    let caption = format!("{}x{} - [ Click / Ctrl+O ]", w, h);
    let caption_widget = Paragraph::new(caption)
        .style(Style::default().fg(p.subtext0).add_modifier(Modifier::DIM))
        .alignment(ratatui::layout::Alignment::Center);
    caption_widget.render(caption_area, buf);
}

pub fn open_image_attachment(index: usize, session_id: Option<&str>) {
    if let Some(path) = resolve_image_file(index, session_id) {
        open_system_file(&path);
    }
}

pub fn open_system_file(path: &Path) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(path).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd").args(["/C", "start", ""]).arg(path).spawn();
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_directory_is_accessible() {
        let dir = cache_dir();
        assert!(dir.exists());
    }

    #[test]
    fn preview_rendering_handles_tiny_areas() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 1, 1));
        let path = PathBuf::from("/nonexistent.png");
        assert_eq!(render_image_preview(&mut buf, Rect::new(0, 0, 1, 1), &path), None);
    }

    #[test]
    fn test_resolve_image_finds_project_images() {
        let res = resolve_image_file(1, None);
        assert!(res.is_some());
        if let Some(ref path) = res {
            assert!(path.exists());
            assert!(path.metadata().map(|m| m.len() > 100).unwrap_or(false));
        }
    }

    #[test]
    fn test_tooltip_rect_calculation() {
        let hover = HoverImage {
            index: 1,
            screen_x: 20,
            screen_y: 10,
            pane_id: 0,
            popup_rect: Rect::new(0, 0, 10, 10),
        };
        let screen = Rect::new(0, 0, 120, 40);
        let rect = tooltip_rect(&hover, screen, None);
        assert_eq!(rect.width, 50);
        assert_eq!(rect.height, 14);
    }

    #[test]
    fn test_preview_renders_valid_image() {
        let res = resolve_image_file(1, None);
        assert!(res.is_some());
        let path = res.unwrap();
        let mut buf = Buffer::empty(Rect::new(0, 0, 30, 10));
        let dims = render_image_preview(&mut buf, Rect::new(0, 0, 30, 10), &path);
        assert!(dims.is_some());
        let (w, h) = dims.unwrap();
        assert!(w > 0 && h > 0);
    }

    #[test]
    fn test_compact_tooltip_bounds() {
        let screen = Rect::new(0, 0, 200, 60);
        let rect = calculate_tooltip_rect(30, 20, 99999, screen, None);
        assert_eq!(rect.width, 50);
        assert_eq!(rect.height, 14);
    }

    #[test]
    fn test_picker_creation_and_protocol() {
        let mut picker = Picker::halfblocks();
        picker.set_protocol_type(ProtocolType::Kitty);
        let img = image::DynamicImage::new_rgb8(1200, 593);
        let proto = picker.new_protocol(img, Size::new(48, 11), Resize::Fit(None)).unwrap();
        assert!(proto.size().width <= 48 && proto.size().height <= 11);
        let mut buf = Buffer::empty(Rect::new(0, 0, 48, 11));
        let widget = RatatuiImageWidget::new(&proto).allow_clipping(true);
        widget.render(Rect::new(0, 0, 48, 11), &mut buf);
        let cell = &buf[(0, 0)];
        assert!(!cell.symbol().is_empty());
    }
}
