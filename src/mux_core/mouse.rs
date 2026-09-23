
use std::io;

use alacritty_terminal::grid::Dimensions;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

pub use super::context_menu_ops::execute_context_menu_action;
pub use super::mouse_scroll::handle_scroll_accum;
use super::modals::{open_context_modal, sync_modal_toggles};
use super::nav::change_pane_cwd;
use super::pane_ops::{active_pane_size, spawn_pane};
use crate::state::AppState;
use crate::ui::modal::{Modal, ModalRow};
use crate::ui::sidebar::SidebarRow;

pub fn content_origin(state: &AppState, area_width: u16) -> (u16, u16) {
    let sidebar_w = if state.sidebar_open && area_width > state.sidebar_w + 8 {
        state.sidebar_w
    } else if area_width > 2 {
        1
    } else {
        0
    };
    (sidebar_w + 1, 2)
}

pub fn build_selected_row(chars: &[(u16, char)], min_x: u16, max_x: u16) -> String {
    let last_col = chars.iter().map(|(cx, _)| *cx).max().unwrap_or(0);
    let row_end = max_x.min(last_col);
    if row_end < min_x {
        return String::new();
    }

    let mut row_buf: Vec<char> = vec![' '; (row_end - min_x + 1) as usize];
    for (cx, c) in chars {
        if *cx >= min_x && *cx <= max_x {
            row_buf[(*cx - min_x) as usize] = *c;
        }
    }
    row_buf
        .into_iter()
        .collect::<String>()
        .trim_end()
        .to_string()
}

pub fn handle_mouse(
    state: &mut AppState,
    terminal: &Terminal<CrosstermBackend<std::io::Stdout>>,
    mouse: MouseEvent,
    command: &str,
    new_tab_cmd: &str,
) -> io::Result<()> {

    if let Some(ref menu) = state.context_menu {
        if matches!(
            mouse.kind,
            MouseEventKind::Down(MouseButton::Left)
                | MouseEventKind::Up(MouseButton::Left)
                | MouseEventKind::Down(MouseButton::Right)
                | MouseEventKind::Up(MouseButton::Right)
        ) {
            let col = mouse.column;
            let row = mouse.row;
            if let Some(action) = menu.hit_test(col, row) {
                state.context_menu = None;
                state.dirty = true;
                execute_context_menu_action(state, action, new_tab_cmd);
                return Ok(());
            }
            state.context_menu = None;
            state.dirty = true;
            return Ok(());
        }
    }

    let is_right_click = match mouse.kind {
        MouseEventKind::Down(MouseButton::Right) | MouseEventKind::Up(MouseButton::Right) => true,
        MouseEventKind::Down(MouseButton::Left) | MouseEventKind::Up(MouseButton::Left) => {
            mouse.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
                || mouse.modifiers.contains(crossterm::event::KeyModifiers::ALT)
                || mouse.modifiers.contains(crossterm::event::KeyModifiers::SUPER)
        }
        _ => false,
    };

    if is_right_click {
        let col = mouse.column;
        let row = mouse.row;
        let sidebar_w = if state.sidebar_open { state.sidebar_w } else { 1 };

        if row == 0 {
            let tab_bar_x = if state.sidebar_open { state.sidebar_w + 1 } else { 1 };
            if col >= tab_bar_x {
                let rel_col = col.saturating_sub(tab_bar_x);
                let w = terminal.size().map(|s| s.width).unwrap_or(80).saturating_sub(tab_bar_x);
                let titles: Vec<String> = state
                    .panes
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        let t = p.lock().unwrap_or_else(|e| e.into_inner()).state.title.clone();
                        if t.is_empty() || t == "commandcode" {
                            format!("Terminal {}", i + 1)
                        } else {
                            t
                        }
                    })
                    .collect();
                let tab_area = Rect::new(0, 0, w, 1);
                let closable = state.panes.len() > 1;
                let tab_geoms = crate::ui::tab_bar::tab_geometries(tab_area, &titles, closable, state.active);
                if let Some(tab_idx) = tab_geoms.iter().position(|g| rel_col >= g.start_x && rel_col < g.start_x + g.width) {
                    if let Some(display_title) = titles.get(tab_idx) {
                        state.context_menu = Some(crate::ui::context_menu::ContextMenu::for_tab(
                            tab_idx,
                            display_title,
                            (col, row),
                        ));
                        state.dirty = true;
                        return Ok(());
                    }
                }
            }
        }

        if state.sidebar_open && col <= sidebar_w {
            if let Some(sidebar_row) = state.sidebar_view.row_at_y(row) {
                if let SidebarRow::Session(sess_idx) = sidebar_row {
                    if let Some(sess) = state.sidebar.sessions.get(sess_idx) {
                        let is_open = state.session_is_open(&sess.id);
                        state.context_menu = Some(crate::ui::context_menu::ContextMenu::for_session(
                            &sess.id,
                            &sess.title,
                            is_open,
                            (col, row),
                        ));
                        state.dirty = true;
                        return Ok(());
                    }
                }
            }
        }

        state.context_menu = Some(crate::ui::context_menu::ContextMenu::for_pane(
            state.active,
            (col, row),
        ));
        state.dirty = true;
        return Ok(());
    }

    if mouse.kind == MouseEventKind::ScrollUp || mouse.kind == MouseEventKind::ScrollDown {
        if state.hover_image.is_some() {
            state.hover_image = None;
            state.dirty = true;
        }
        if let Some(p) = state.picker.as_mut() {
            if mouse.kind == MouseEventKind::ScrollUp {
                p.picker.move_selection(-1);
            } else {
                p.picker.move_selection(1);
            }
            state.dirty = true;
            return Ok(());
        }
        let col = mouse.column;
        let row = mouse.row;
        let over_sidebar = state.sidebar_open && (col as u16) < state.sidebar_w;
        let over_tab_bar = !over_sidebar && row == 0;

        if over_sidebar {
            if mouse.kind == MouseEventKind::ScrollUp {
                state.sidebar.prev();
            } else {
                state.sidebar.next();
            }
        } else if over_tab_bar {
            if mouse.kind == MouseEventKind::ScrollUp {
                let prev = if state.active == 0 {
                    state.panes.len().saturating_sub(1)
                } else {
                    state.active - 1
                };
                state.focus_tab(prev);
            } else if !state.panes.is_empty() {
                let next = (state.active + 1) % state.panes.len();
                state.focus_tab(next);
            }
            state.sidebar_focus = false;
        } else if let Some(pane) = state.panes.get(state.active) {
            let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());
            let delta = if mouse.kind == MouseEventKind::ScrollUp {
                3
            } else {
                -3
            };
            p.scroll_display(delta);
        }
        return Ok(());
    }

    if mouse.kind == MouseEventKind::Drag(MouseButton::Left) {
        let col = mouse.column;
        let row = mouse.row;

        if state.resizing_sidebar {
            state.resizing_sidebar_dragged = true;
            if !state.sidebar_open {
                if col >= 8 {
                    state.sidebar_open = true;
                    state.sidebar_w = col.clamp(6, 50);
                    state.dirty = true;
                }
            } else {
                if col < 4 {
                    state.sidebar_open = false;
                    state.dirty = true;
                } else {
                    state.sidebar_w = col.clamp(6, 50);
                    state.dirty = true;
                }
            }
            return Ok(());
        }

        if state.resizing_panel {
            state.resizing_panel_dragged = true;
            let size = terminal.size()?;
            let new_w = size.width.saturating_sub(col);
            if !state.panel_sidebar_open {
                if new_w >= 16 {
                    state.panel_sidebar_open = true;
                    state.panel_sidebar_w = new_w
                        .clamp(crate::state::PANEL_SIDEBAR_MIN, crate::state::PANEL_SIDEBAR_MAX);
                    state.dirty = true;
                }
            } else {
                if new_w < 8 {
                    state.panel_sidebar_open = false;
                    state.panel_maximized = false;
                    state.panel_focused = false;
                    state.dirty = true;
                } else {
                    state.panel_sidebar_w = new_w
                        .clamp(crate::state::PANEL_SIDEBAR_MIN, crate::state::PANEL_SIDEBAR_MAX);
                    state.dirty = true;
                }
            }
            state.sync_sidebar_panel();
            return Ok(());
        }

        let (content_left, content_top) = content_origin(state, terminal.size().map(|s| s.width).unwrap_or(80));
        if col >= content_left {
            if let Some(pane) = state.panes.get(state.active) {
                let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());
                let metrics = p.scroll_metrics();
                let vx = col.saturating_sub(content_left);
                let viewport = p.term.screen_lines() as u16;

                if row < content_top {
                    if metrics.offset_from_bottom < metrics.max_offset_from_bottom {
                        p.scroll_display(1);
                    }
                } else {
                    let vy = row.saturating_sub(content_top);
                    if vy >= viewport {

                        if metrics.offset_from_bottom > 0 {
                            p.scroll_display(-1);
                        }
                    } else {

                        let metrics = p.scroll_metrics();
                        if let Some(ref mut sel) = p.state.selection {
                            sel.drag(vy, vx, metrics);
                        } else {
                            p.state.selection =
                                Some(crate::selection::Selection::anchor(vy, vx, metrics));
                        }
                    }
                }
            }
        }
        return Ok(());
    }

    if mouse.kind == MouseEventKind::Up(MouseButton::Left) {

        if state.resizing_panel {
            state.resizing_panel = false;
            let was_dragged = state.resizing_panel_dragged;
            state.resizing_panel_dragged = false;
            let size = terminal.size()?;
            let right_grip_col = size.width.saturating_sub(1);
            if !was_dragged && state.panel_active && !state.panel_sidebar_open && mouse.column >= right_grip_col.saturating_sub(1) {
                state.panel_sidebar_open = true;
                if state.panel_sidebar_w < crate::state::PANEL_SIDEBAR_MIN {
                    state.panel_sidebar_w = 32;
                }
            }
            state.sync_sidebar_panel();
            state.dirty = true;
            return Ok(());
        }
        if state.resizing_sidebar {
            state.resizing_sidebar = false;
            let was_dragged = state.resizing_sidebar_dragged;
            state.resizing_sidebar_dragged = false;
            if !was_dragged && !state.sidebar_open && mouse.column <= 2 {
                state.sidebar_open = true;
                if state.sidebar_w < 18 {
                    state.sidebar_w = 25;
                }
                state.sidebar_focus = true;
            }
            state.dirty = true;
            return Ok(());
        }
        if let Some(pane) = state.panes.get(state.active) {
            let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(ref mut sel) = p.state.selection {
                if sel.finish() {
                    let ((sr, sc), (er, ec)) = sel.ordered();

                    let grid = p.term.grid();
                    let hist_len = grid.history_size();

                    let start = alacritty_terminal::index::Point::new(
                        alacritty_terminal::index::Line(-(hist_len as i32)),
                        alacritty_terminal::index::Column(0),
                    );
                    let mut lines_map: std::collections::BTreeMap<u32, Vec<(u16, char)>> =
                        std::collections::BTreeMap::new();
                    for item in grid.iter_from(start) {

                        if item
                            .cell
                            .flags
                            .intersects(alacritty_terminal::term::cell::Flags::WIDE_CHAR_SPACER)
                        {
                            continue;
                        }
                        let abs_row = (item.point.line.0 as i32 + hist_len as i32).max(0) as u32;
                        lines_map
                            .entry(abs_row)
                            .or_default()
                            .push((item.point.column.0 as u16, item.cell.c));
                    }

                    let max_cols = p.term.columns() as u16;
                    let mut lines_text: Vec<String> = Vec::new();
                    for r in sr..=er {
                        if let Some(chars) = lines_map.get(&r) {
                            let min_x = if r == sr { sc } else { 0 };
                            let max_x = if r == er { ec } else { max_cols };
                            lines_text.push(build_selected_row(chars, min_x, max_x));
                        }
                    }
                    let selected_text = lines_text.join("\n");
                    if !selected_text.trim().is_empty() {

                        super::nav::copy_to_clipboard(&selected_text);
                    }
                } else if sel.was_just_click() {
                    let (click_vy, click_vx) = sel.viewport_click;
                    let line_str = p.viewport_line_text(click_vy as usize);
                    let pending_cwd = p.state.pending_cwd.clone();
                    p.state.selection = None;
                    if let Some(hit) = crate::ui::links::link_at(&line_str, click_vx as usize) {
                        match hit {
                            crate::ui::links::Hit::Url(url) => {
                                crate::ui::links::open_url(&url);
                            }
                            crate::ui::links::Hit::FilePath { path, line, col: _ } => {
                                crate::ui::links::open_file_in_editor(&path, line, pending_cwd.as_deref());
                            }
                            crate::ui::links::Hit::ImageAttachment { index } => {
                                crate::ui::image_tooltip::open_image_attachment(index, p.state.session_id.as_deref());
                            }
                        }
                    }
                }
            }
        }
        return Ok(());
    }

    if mouse.kind == MouseEventKind::Moved {
        let col = mouse.column;
        let row = mouse.row;
        let sidebar_w = if state.sidebar_open { state.sidebar_w } else { 1 };
        if state.sidebar_open && (col as i32 - sidebar_w as i32).abs() <= 1 && row > 0 {
            state.hover_divider = Some(crate::ui::borders::HoverDivider {
                axis: crate::ui::borders::DividerAxis::Vertical,
                line: sidebar_w,
                span: (1, terminal.size()?.height),
            });
            state.dirty = true;
        } else if !state.sidebar_open && col <= 1 && row > 0 {
            state.hover_divider = Some(crate::ui::borders::HoverDivider {
                axis: crate::ui::borders::DividerAxis::Vertical,
                line: 1,
                span: (1, terminal.size()?.height),
            });
            state.dirty = true;
        } else if state.hover_divider.is_some() {
            state.hover_divider = None;
            state.dirty = true;
        }

        let mut found_image_hover = None;
        let area_width = terminal.size().map(|s| s.width).unwrap_or(80);
        let (content_left, content_top) = content_origin(state, area_width);
        if col >= content_left && row >= content_top {
            if let Some(pane) = state.panes.get(state.active) {
                if let Ok(p) = pane.lock() {
                    let pane_w = p.term.columns() as u16;
                    let pane_h = p.term.screen_lines() as u16;
                    let vx = col.saturating_sub(content_left);
                    let vy = row.saturating_sub(content_top);
                    if vx < pane_w && vy < pane_h {
                        let line_str = p.viewport_line_text(vy as usize);
                        if let Some((start_col, _end_col, index)) =
                            crate::ui::links::image_token_at(&line_str, vx as usize)
                        {
                            let anchor_x = content_left + start_col as u16;
                            let anchor_y = row;
                            if let Some(ref current) = state.hover_image {
                                if current.index == index
                                    && current.screen_x == anchor_x
                                    && current.screen_y == anchor_y
                                    && current.pane_id == state.active
                                {
                                    found_image_hover = Some(current.clone());
                                }
                            }
                            if found_image_hover.is_none() {
                                let session_id = p.state.session_id.clone();
                                let screen_h = terminal.size().map(|s| s.height).unwrap_or(24);
                                let screen = ratatui::layout::Rect::new(0, 0, area_width, screen_h);
                                let popup = crate::ui::image_tooltip::calculate_tooltip_rect(
                                    anchor_x,
                                    anchor_y,
                                    index,
                                    screen,
                                    session_id.as_deref(),
                                );
                                found_image_hover = Some(crate::state::HoverImage {
                                    index,
                                    screen_x: anchor_x,
                                    screen_y: anchor_y,
                                    pane_id: state.active,
                                    popup_rect: popup,
                                });
                            }
                        }
                    }
                }
            }
        }

        if found_image_hover.is_none() {
            if let Some(ref current) = state.hover_image {
                let rect = current.popup_rect;
                if col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height {
                    found_image_hover = Some(current.clone());
                }
            }
        }

        if state.hover_image != found_image_hover {
            state.hover_image = found_image_hover;
            state.dirty = true;
        }
    }

    if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
        let col = mouse.column;
        let row = mouse.row;
        let h = terminal.size()?.height;
        let sidebar_w = if state.sidebar_open { state.sidebar_w } else { 1 };

        if let Some(ref hover) = state.hover_image {
            let rect = hover.popup_rect;
            if col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height {
                let session_id = state
                    .panes
                    .get(state.active)
                    .and_then(|p| p.lock().ok())
                    .and_then(|p| p.state.session_id.clone());
                crate::ui::image_tooltip::open_image_attachment(hover.index, session_id.as_deref());
                state.hover_image = None;
                state.dirty = true;
                return Ok(());
            } else {
                state.hover_image = None;
                state.dirty = true;
            }
        }

        if let Some(ref menu) = state.context_menu {
            if let Some(action) = menu.hit_test(col, row) {
                state.context_menu = None;
                state.dirty = true;
                execute_context_menu_action(state, action, new_tab_cmd);
                return Ok(());
            }
            state.context_menu = None;
            state.dirty = true;
            return Ok(());
        }

        if state.cmd_inspect.is_some() {
            state.cmd_inspect = None;
            state.dirty = true;
            return Ok(());
        }

        if state.switcher.is_some() {
            state.switcher = None;
            state.dirty = true;
            return Ok(());
        }

        if state.sidebar_open && col >= sidebar_w.saturating_sub(1) && col <= sidebar_w + 2 && row > 0 {
            state.resizing_sidebar = true;
            state.resizing_sidebar_dragged = false;
            state.dirty = true;
            return Ok(());
        }

        if !state.sidebar_open && col <= 1 && row > 0 {
            state.resizing_sidebar = true;
            state.resizing_sidebar_dragged = false;
            state.dirty = true;
            return Ok(());
        }

        if state.panel_active && !state.panel_maximized {
            let size = terminal.size()?;
            let area = Rect::new(0, 0, size.width, size.height);
            if state.panel_sidebar_open {
                let sidebar_w = if state.sidebar_open && area.width > state.sidebar_w + 8 {
                    state.sidebar_w
                } else if area.width > 2 {
                    1
                } else {
                    0
                };
                let avail = area.width.saturating_sub(sidebar_w);
                let w = state.panel_sidebar_w.min(avail / 2);
                let panel_left = area.width.saturating_sub(w);
                if col >= panel_left.saturating_sub(2) && col <= panel_left + 1 && row > 0 {
                    state.resizing_panel = true;
                    state.resizing_panel_dragged = false;
                    state.dirty = true;
                    return Ok(());
                }
            } else {
                let grip_col = area.width.saturating_sub(1);
                if col >= grip_col.saturating_sub(1) && row > 0 {
                    state.resizing_panel = true;
                    state.resizing_panel_dragged = false;
                    state.dirty = true;
                    return Ok(());
                }
            }
        }

        if state.panel_sidebar_open || state.panel_maximized {
            let size = terminal.size()?;
            let area = Rect::new(0, 0, size.width, size.height);
            if crate::mux_core::mouse_modals::handle_panel_click(state, col, row, area) {
                return Ok(());
            } else {
                state.panel_focused = false;
                state.dirty = true;
            }
        }

        if state.picker.is_some() {
            let size = terminal.size()?;
            let area = Rect::new(0, 0, size.width, size.height);
            let visible = state
                .picker
                .as_ref()
                .map(|p| p.picker.filtered_indices().len().min(14).max(4))
                .unwrap_or(6);
            let picker_w = 68u16.min(area.width.saturating_sub(4)).max(56);
            if let Some(pop) = crate::ui::modal::modal_rect(area, visible + 4, 0, picker_w) {
                if col >= pop.x
                    && col < pop.x + pop.width
                    && row >= pop.y
                    && row < pop.y + pop.height
                {
                    let rel_y = row.saturating_sub(pop.y);
                    if rel_y == 2 {
                        let rel_x = col.saturating_sub(pop.x + 1) as usize;
                        if rel_x < 12 {
                            if let Some(p) = state.picker.as_mut() {
                                p.picker.set_category(0);
                            }
                        } else if rel_x < 24 {
                            if let Some(p) = state.picker.as_mut() {
                                p.picker.set_category(1);
                            }
                        } else if rel_x < 44 {
                            if let Some(p) = state.picker.as_mut() {
                                p.picker.set_category(2);
                            }
                        } else {
                            if let Some(p) = state.picker.as_mut() {
                                p.picker.set_category(3);
                            }
                        }
                        return Ok(());
                    } else if rel_y >= 4 {
                        let row_click = (rel_y - 4) as usize;
                        if let Some(p) = state.picker.as_mut() {
                            let indices = p.picker.filtered_indices();
                            let visible_count = visible;
                            let scroll_offset = if p.picker.selected < visible_count {
                                0
                            } else {
                                p.picker.selected.saturating_sub(visible_count - 1)
                            };
                            let target_idx = scroll_offset + row_click;
                            if target_idx < indices.len() {
                                let opt_idx = indices[target_idx];
                                let target_row = p.row_idx;
                                if let Some(ref mut modal) = state.active_modal {
                                    modal.selected = target_row;
                                    modal.select_option(opt_idx);
                                    modal.dirty = true;
                                    modal.save();
                                }
                                state.picker = None;
                                sync_modal_toggles(state);
                                return Ok(());
                            }
                        }
                    }
                    return Ok(());
                }
            }
            state.picker = None;
            return Ok(());
        }

        if state.active_modal.is_some() {
            let size = terminal.size()?;
            let area = Rect::new(0, 0, size.width, size.height);
            crate::mux_core::mouse_modals::handle_active_modal_click(state, col, row, area);
            return Ok(());
        }

        if state.sidebar_open && col <= sidebar_w {
            state.sidebar_focus = true;
            let hit = state
                .sidebar_view
                .zone_at(col, row)
                .or_else(|| state.sidebar_view.row_at_y(row));
            crate::ipc::log_append("resize.log", &format!("sidebar_click: ({col},{row}) -> {hit:?}"));
            if let Some(sidebar_row) = hit {
                super::mouse_sidebar::handle_sidebar_click(state, sidebar_row, command, new_tab_cmd)?;
            }
            return Ok(());
        } else {
            state.sidebar_focus = false;
            if row >= h.saturating_sub(1) {
                open_context_modal(state);
                return Ok(());
            }
            if row == 0 {
                let size = terminal.size()?;
                let closable = state.panes.len() > 1;
                let titles: Vec<String> = state
                    .panes
                    .iter()
                    .map(|p| p.lock().unwrap_or_else(|e| e.into_inner()).state.title.clone())
                    .collect();
                let tab_area = Rect::new(sidebar_w, 0, size.width.saturating_sub(sidebar_w), 1);
                let geoms = crate::ui::tab_bar::tab_geometries(tab_area, &titles, closable, state.active);
                let mut clicked_tab: Option<(usize, bool)> = None;
                let mut clicked_plus = false;
                let active_idx = if state.active < geoms.len() { state.active } else { 0 };

                for (idx, geom) in geoms.iter().enumerate() {
                    if col >= geom.start_x && col < geom.start_x + geom.width {
                        if idx == active_idx {
                            let plus_col = geom.start_x + geom.width.saturating_sub(4);
                            if col >= plus_col && col <= plus_col + 2 {
                                clicked_plus = true;
                                break;
                            }
                        }
                        let close_zone = if idx == active_idx {
                            geom.body_x + geom.body_len.saturating_sub(5)
                        } else {
                            geom.body_x + geom.body_len.saturating_sub(3)
                        };
                        if closable && col >= close_zone && col < geom.body_x + geom.body_len {
                            clicked_tab = Some((idx, true));
                        } else {
                            clicked_tab = Some((idx, false));
                        }
                        break;
                    }
                }

                let plus_x = geoms
                    .last()
                    .map(|g| g.start_x + g.width)
                    .unwrap_or(tab_area.left() + 1);
                if !clicked_plus && clicked_tab.is_none() && col >= plus_x && col < plus_x + 4 {
                    clicked_plus = true;
                }

                if let Some((idx, is_close)) = clicked_tab {
                    if is_close && state.panes.len() > 1 {

                        if state
                            .active_modal
                            .as_ref()
                            .map(|m| m.id == "confirm_close")
                            .unwrap_or(false)
                            && state.confirm_close_idx == Some(idx)
                        {

                        } else {

                            let busy = state.panes.get(idx).map(|p| {
                                p.lock()
                                    .map(|g| {
                                        g.state.agent_state != crate::agent_state::AgentState::Idle
                                    })
                                    .unwrap_or(false)
                            });
                            if busy == Some(true) {
                                let mut m = Modal::new(
                                    "confirm_close",
                                    format!("Close Terminal #{}?", idx + 1),
                                );
                                m.rows.push(ModalRow::Info(
                                    "This terminal has activity in progress.".into(),
                                ));
                                m.rows.push(ModalRow::Choice {
                                    key: "confirm".into(),
                                    label: "Close anyway?".into(),
                                    options: vec![
                                        ("Close".into(), "close".into(), "danger".into()),
                                        ("Cancel".into(), "cancel".into(), "action".into()),
                                    ],

                                    current: 0,
                                    searchable: false,
                                    color: String::new(),
                                });
                                m.hints.push(("Enter".into(), "Confirm".into()));
                                m.hints.push(("Esc".into(), "Cancel".into()));
                                state.confirm_close_idx = Some(idx);
                                state.active_modal = Some(m);
                            } else {
                                state.close_pane(idx);
                            }
                        }
                    } else {
                        let now = std::time::Instant::now();
                        let is_double_click = match (state.last_click_tab, state.last_click_time) {
                            (Some(last_idx), Some(last_time))
                                if last_idx == idx
                                    && now.duration_since(last_time)
                                        < std::time::Duration::from_millis(400) =>
                            {
                                true
                            }
                            _ => false,
                        };
                        state.last_click_tab = Some(idx);
                        state.last_click_time = Some(now);

                        if is_double_click {
                            let current_title =
                                state.panes[idx].lock().unwrap_or_else(|e| e.into_inner()).state.title.clone();
                            let mut m =
                                Modal::new("rename_tab", format!("Rename Terminal #{}", idx + 1));
                            m.rows.push(ModalRow::TextInput {
                                key: "title".into(),
                                label: "Title".into(),
                                value: if current_title == "commandcode" {
                                    String::new()
                                } else {
                                    current_title
                                },
                            });
                            m.rows.push(ModalRow::Info(
                                "Type new name, press ENTER to save or ESC to cancel".into(),
                            ));
                            state.active_modal = Some(m);
                            state.sidebar_focus = false;
                        } else {
                            state.focus_tab(idx);
                            state.sidebar_focus = false;
                        }
                    }
                } else if clicked_plus {
                    let (cols, rows) = active_pane_size(state);
                    let mut tab_cmd = new_tab_cmd.to_string();
                    if state.sidebar.yolo_mode && !tab_cmd.contains("--yolo") {
                        tab_cmd.push_str(" --yolo");
                    }
                    if spawn_pane(state, &tab_cmd, cols, rows).is_ok() {
                        state.active = state.panes.len() - 1;
                        state.refresh_mods_now();
                        state.sidebar_focus = false;
                    }
                }
            } else {
                let area_width = terminal.size().map(|s| s.width).unwrap_or(80);
                let (content_left, content_top) = content_origin(state, area_width);
                if row >= content_top && col >= content_left {
                    if let Some(pane) = state.panes.get(state.active) {
                        let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());

                        if let Some((ix, iy)) = p.state.banner_folder_icon {
                            if col as u16 == ix && row as u16 == iy {
                                drop(p);
                                change_pane_cwd(state);
                                return Ok(());
                            }
                        }

                        let (pane_w, pane_h) =
                            (p.term.columns() as u16, p.term.screen_lines() as u16);
                        let gutter_col = content_left.saturating_add(pane_w).saturating_sub(1);
                        let metrics = p.scroll_metrics();
                        if col == gutter_col
                            && metrics.max_offset_from_bottom > 0
                            && pane_h > 2
                        {
                            let track_top = content_top;
                            let track_bottom = content_top.saturating_add(pane_h).saturating_sub(1);

                            let total_rows = metrics.max_offset_from_bottom + metrics.viewport_rows;
                            let track_h = pane_h as i64;
                            let clicked_anchor = p.state.prompt_anchors.iter().rev().find(|&&a| {
                                let off = (a as i64 + metrics.max_offset_from_bottom as i64)
                                    .clamp(0, total_rows as i64);
                                let r = (off * track_h) / total_rows as i64;
                                (track_top as i64 + r) == row as i64
                            }).copied();
                            if let Some(anchor) = clicked_anchor {

                                let target = (metrics.max_offset_from_bottom as i64
                                    + anchor as i64)
                                    .clamp(0, metrics.max_offset_from_bottom as i64)
                                    as usize;
                                let current = p.term.grid().display_offset();
                                let delta = target as i64 - current as i64;
                                p.scroll_display(delta as i32);
                            } else {
                                let rel = (row as f64 - track_top as f64) / (track_bottom as f64 - track_top as f64);
                                p.scroll_to_fraction(rel);
                            }
                            p.state.last_manual_scroll = Some(std::time::Instant::now());
                            drop(p);
                            state.dirty = true;
                            return Ok(());
                        }
                        let metrics = p.scroll_metrics();
                        let vx = col.saturating_sub(content_left);
                        let vy = row.saturating_sub(content_top);
                        p.state.selection = Some(crate::selection::Selection::anchor(vy, vx, metrics));

                        p.state.last_manual_scroll = Some(std::time::Instant::now());
                    }
                }
            }
        }
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_origin_with_sidebar_open_and_closed() {
        let mut state = AppState::new(crate::ui::sidebar::Sidebar::load());
        state.sidebar_open = true;
        state.sidebar_w = 25;

        let (left, top) = content_origin(&state, 100);
        assert_eq!(left, 26);
        assert_eq!(top, 2);

        state.sidebar_open = false;
        let (left, top) = content_origin(&state, 100);
        assert_eq!(left, 2);
        assert_eq!(top, 2);
    }

    #[test]
    fn test_build_selected_row_with_spaces_and_bounds() {
        let chars = vec![(0, 'H'), (1, 'e'), (2, 'l'), (3, 'l'), (4, 'o'), (10, 'W')];
        let row = build_selected_row(&chars, 0, 10);
        assert_eq!(row, "Hello     W");

        let partial = build_selected_row(&chars, 2, 4);
        assert_eq!(partial, "llo");
    }
}

