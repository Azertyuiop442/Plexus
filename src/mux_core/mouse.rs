
use std::io;

use alacritty_terminal::grid::Dimensions;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

use super::modals::{
    open_ai_prefs_modal, open_all_sessions_modal, open_context_modal, open_full_config_modal,
    open_mod_config_modal, open_update_progress_modal, sync_modal_toggles,
};
use super::nav::{change_pane_cwd, reload_mux, send_slash_command};
use super::pane_ops::{active_pane_size, spawn_pane};
use crate::state::AppState;
use crate::ui::modal::{Modal, ModalRow};
use crate::ui::sidebar::LiveBlock;
use crate::ui::sidebar::{session_resumable, SettingsSubMenu, SidebarRow};

pub fn content_origin(state: &AppState, area_width: u16) -> (u16, u16) {
    let sidebar_w = if state.sidebar_open && area_width > state.sidebar_w + 8 {
        state.sidebar_w
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
        let sidebar_w = if state.sidebar_open { state.sidebar_w } else { 0 };

        if row == 0 {
            let tab_bar_x = if state.sidebar_open { state.sidebar_w + 1 } else { 0 };
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
                let tab_geoms = crate::ui::tab_bar::tab_geometries(tab_area, &titles, closable);
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
            state.sidebar_w = col.clamp(18, 50);
            state.dirty = true;
            return Ok(());
        }

        if state.resizing_panel {
            let size = terminal.size()?;
            let new_w = size.width.saturating_sub(col);
            state.panel_sidebar_w = new_w
                .clamp(crate::state::PANEL_SIDEBAR_MIN, crate::state::PANEL_SIDEBAR_MAX);
            state.dirty = true;
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
            state.dirty = true;
            return Ok(());
        }
        if state.resizing_sidebar {
            state.resizing_sidebar = false;
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
        let sidebar_w = state.sidebar_w;
        if state.sidebar_open && (col as i32 - sidebar_w as i32).abs() <= 1 && row > 0 {
            state.hover_divider = Some(crate::ui::borders::HoverDivider {
                axis: crate::ui::borders::DividerAxis::Vertical,
                line: sidebar_w,
                span: (1, terminal.size()?.height),
            });
            state.dirty = true;
        } else if state.hover_divider.is_some() {
            state.hover_divider = None;
            state.dirty = true;
        }
    }

    if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
        let col = mouse.column;
        let row = mouse.row;
        let h = terminal.size()?.height;
        let sidebar_w = state.sidebar_w;

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
            state.dirty = true;
            return Ok(());
        }

        if (state.panel_sidebar_open || state.panel_maximized) && !state.panel_maximized {
            let size = terminal.size()?;
            let area = Rect::new(0, 0, size.width, size.height);
            let panel_left = area.width.saturating_sub(state.panel_sidebar_w);
            if col >= panel_left.saturating_sub(2) && col <= panel_left + 1 && row > 0 {
                state.resizing_panel = true;
                state.dirty = true;
                return Ok(());
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
            let (modal_rows, modal_cmds) = state
                .active_modal
                .as_ref()
                .map(|m| (m.rows.len(), m.commands.len()))
                .unwrap_or((6, 0));
            if let Some(pop) = crate::ui::modal::modal_rect(area, modal_rows, modal_cmds, 56) {
                if col >= pop.x
                    && col < pop.x + pop.width
                    && row >= pop.y
                    && row < pop.y + pop.height
                {
                    let rel_y = row.saturating_sub(pop.y);
                    if rel_y == 1 {
                        let rel_x = col.saturating_sub(pop.x + 2) as usize;
                        if rel_x < 6 {
                            if let Some(p) = state.picker.as_mut() {
                                p.picker.set_category(0);
                            }
                        } else if rel_x < 14 {
                            if let Some(p) = state.picker.as_mut() {
                                p.picker.set_category(1);
                            }
                        } else {
                            if let Some(p) = state.picker.as_mut() {
                                p.picker.set_category(2);
                            }
                        }
                        return Ok(());
                    } else if rel_y >= 2 {
                        let row_click = (rel_y - 2) as usize;
                        if let Some(p) = state.picker.as_mut() {
                            let indices = p.picker.filtered_indices();
                            if row_click < indices.len() {
                                let opt_idx = indices[row_click];
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
                }
            }
            state.picker = None;
            return Ok(());
        }

        if state.active_modal.is_some() {
            let size = terminal.size()?;
            let area = Rect::new(0, 0, size.width, size.height);
            let (modal_rows, modal_cmds, content_w) = state
                .active_modal
                .as_ref()
                .map(|m| {
                    let rows = if m.page_size > 0 {
                        m.visible_rows()
                            .iter()
                            .map(|r| crate::ui::modal::row_wrapped_lines(r, 78).max(1))
                            .sum::<u16>() as usize
                    } else {
                        m.rows
                            .iter()
                            .map(|r| crate::ui::modal::row_wrapped_lines(r, 78).max(1))
                            .sum::<u16>() as usize
                    };
                    let cmds = if m.steps.is_empty() { m.commands.len() } else { 0 };
                    let tw = if !m.steps.is_empty() {
                        m.steps
                            .iter()
                            .map(|s| (crate::ui::text::width(&s.title) + 4) as u16)
                            .sum::<u16>()
                            .saturating_add(4)
                    } else {
                        0
                    };
                    let cw = m
                        .visible_rows()
                        .iter()
                        .map(crate::ui::modal::row_content_width)
                        .max()
                        .unwrap_or(56)
                        .max(tw)
                        .clamp(56, 120);
                    (rows, cmds, cw)
                })
                .unwrap_or((0, 0, 56));
            if let Some(pop) = crate::ui::modal::modal_rect(area, modal_rows, modal_cmds, content_w) {
                if col >= pop.x
                    && col < pop.x + pop.width
                    && row >= pop.y
                    && row < pop.y + pop.height
                {
                    let rel_y = row.saturating_sub(pop.y);
                    if let Some(ref mut modal) = state.active_modal {
                        if !modal.steps.is_empty() && rel_y == 1 {

                            let inner_w = pop.width.saturating_sub(2) as usize;
                            let n_steps = modal.steps.len();
                            let rel_x = col.saturating_sub(pop.x + 1) as usize;
                            let target_step = (rel_x * n_steps) / inner_w.max(1);
                            if target_step < n_steps && target_step != modal.current_step {
                                modal.steps[modal.current_step].rows = modal.rows.clone();
                                modal.current_step = target_step;
                                modal.rows = modal.steps[modal.current_step].rows.clone();
                                modal.select_first_selectable();
                            }
                            return Ok(());
                        }

                        let content_offset = if !modal.steps.is_empty() { 3 } else { 2 };
                        if rel_y >= content_offset {
                            let row_within = (rel_y - content_offset) as usize;
                            if row_within < modal.visible_rows().len() {
                                modal.selected = modal.page_start() + row_within;
                                if modal.selected_is_searchable_choice() {
                                    let idx = modal.selected;
                                    if let Some(crate::ui::modal::ModalRow::Choice { options, current, .. }) = modal.rows.get(idx) {
                                        let current_value = options.get(*current).map(|(_, v, _)| v.clone()).unwrap_or_default();
                                        let picker = crate::ui::modal::ModelPicker::new(options.clone(), current_value);
                                        state.picker = Some(crate::state::PickerState { row_idx: idx, picker });
                                    }
                                } else {
                                    modal.cycle_selected();
                                }
                                if !modal.steps.is_empty() && modal.current_step < modal.steps.len() {
                                    modal.steps[modal.current_step].rows = modal.rows.clone();
                                }
                            } else if modal.steps.is_empty() && row_within > modal.visible_rows().len() {
                                let cmd_idx = row_within - modal.visible_rows().len() - 1;
                                if cmd_idx < modal.commands.len() {
                                    let (name, _desc) = modal.commands[cmd_idx].clone();
                                    let mut bytes = vec![0x15u8];
                                    bytes.extend_from_slice(format!("/{}\r", name).as_bytes());
                                    if let Some(pane) = state.panes.get(state.active) {
                                        pane.lock().unwrap_or_else(|e| e.into_inner()).write_input(&bytes);
                                    }
                                }
                            }
                        }
                    }
                    sync_modal_toggles(state);
                    return Ok(());
                }
            }
            sync_modal_toggles(state);
            state.active_modal = None;
            return Ok(());
        }

        if col <= sidebar_w {
            state.sidebar_focus = true;
            if let Some(sidebar_row) = state
                .sidebar_view
                .zone_at(col, row)
                .or_else(|| state.sidebar_view.row_at_y(row))
            {

                state.sidebar.selected = state.sidebar.selection_index(sidebar_row);
                match sidebar_row {
                    SidebarRow::NewSession => {
                        let (cols, rows) = active_pane_size(state);

                        let mut tab_cmd = new_tab_cmd.to_string();
                        if state.sidebar.yolo_mode && !tab_cmd.contains("--yolo") {
                            tab_cmd.push_str(" --yolo");
                        }
                        if spawn_pane(state, &tab_cmd, cols, rows).is_ok() {
                            state.active = state.panes.len() - 1;
                            state.sidebar_focus = false;
                        }
                    }
                    SidebarRow::Session(i) => {

                        let already_open = state.sidebar.sessions.get(i).and_then(|s| {
                            if state.session_is_open(&s.id) {
                                return state.panes.iter().position(|p| {
                                    p.lock()
                                        .map(|g| g.is_session_live(&s.id))
                                        .unwrap_or(false)
                                });
                            }
                            None
                        });
                        if let Some(pane_idx) = already_open {
                            state.focus_tab(pane_idx);
                            state.sidebar_focus = false;
                            return Ok(());
                        }
                        let mut cmd = if let Some(session) = state.sidebar.sessions.get(i) {

                            let session_id = session.id.clone();
                            if !session_resumable(&session_id, &state.sidebar.project) {
                                state.sidebar.remove_session_by_id(&session_id);
                                command.to_string()
                            } else {

                                let cwd = state.sidebar.project_cwd.clone();
                                super::nav::session_launch_cmd(&cwd, &session_id)
                                    .unwrap_or_else(|| command.to_string())
                            }
                        } else {
                            command.to_string()
                        };

                        if state.sidebar.yolo_mode && !cmd.contains("--yolo") {
                            cmd.push_str(" --yolo");
                        }
                        let (cols, rows) = active_pane_size(state);
                        if spawn_pane(state, &cmd, cols, rows).is_ok() {
                            state.active = state.panes.len() - 1;
                            state.sidebar_focus = false;
                        }
                    }
                    SidebarRow::UsageCarousel => {
                        state.sidebar.next_usage_tab();
                    }
                    SidebarRow::UsagePrev => {
                        state.sidebar.prev_usage_tab();
                    }
                    SidebarRow::UsageNext => {
                        state.sidebar.next_usage_tab();
                    }
                    SidebarRow::MoreSessions => {
                        open_all_sessions_modal(state);
                    }
                    SidebarRow::NavPreferences => {
                        state.sidebar.open_submenu(SettingsSubMenu::Preferences);
                    }
                    SidebarRow::NavModConfig => {
                        state.sidebar.open_submenu(SettingsSubMenu::ModConfig);
                    }
                    SidebarRow::NavAIPrefs => {
                        open_ai_prefs_modal(state);
                    }
                    SidebarRow::NavBack => {
                        state.sidebar.open_submenu(SettingsSubMenu::Main);
                    }
                    SidebarRow::PrefFullConfig => {
                        open_full_config_modal(state);
                    }
                    SidebarRow::PrefAutoRetry => {
                        crate::ui::modal::open_auto_retry_modal(state);
                    }
                    SidebarRow::PrefSkills => {
                        crate::ui::modal::open_skills_modal(state);
                    }
                    SidebarRow::PrefSkillInjection => {
                        let mut prefs = crate::prefs::Prefs::load();
                        prefs.skill_injection = !prefs.skill_injection;
                        prefs.skills.injection_enabled = prefs.skill_injection;
                        let _ = prefs.save();
                        state.sidebar.skill_injection = prefs.skill_injection;
                        state.dirty = true;
                    }
                    SidebarRow::PrefYolo => {
                        state.sidebar.yolo_mode = !state.sidebar.yolo_mode;

                        state.sidebar_focus = false;
                    }
                    SidebarRow::PrefShowUsage => {
                        state.sidebar.show_usage = !state.sidebar.show_usage;
                        state.sidebar.rebuild_rows();
                        state.dirty = true;
                    }
                    SidebarRow::PrefSounds => {
                        crate::ui::modal::open_sounds_modal(state);
                    }
                    SidebarRow::ModConfig(idx) => {
                        open_mod_config_modal(state, idx);
                    }
                    SidebarRow::Reload => {
                        reload_mux();
                    }
                    SidebarRow::Twitter => {
                        #[cfg(target_os = "macos")]
                        {
                            let _ = std::process::Command::new("open")
                                .arg("https://x.com/astra442")
                                .spawn();
                        }
                        #[cfg(target_os = "linux")]
                        {
                            let _ = std::process::Command::new("xdg-open")
                                .arg("https://x.com/astra442")
                                .spawn();
                        }
                        #[cfg(target_os = "windows")]
                        {
                            let _ = std::process::Command::new("cmd")
                                .args(["/C", "start", "", "https://x.com/astra442"])
                                .spawn();
                        }
                    }
                    SidebarRow::Update => {
                        state.sidebar.available_update = None;
                        open_update_progress_modal(state, "Starting update...", 5, 100);
                        crate::update::perform_update_with_events(state.events.clone());
                        state.dirty = true;
                    }
                    SidebarRow::LiveBlockOpen(_) => {

                        let open_path = state
                            .sidebar
                            .live_blocks
                            .iter()
                            .find(|b| b.open_path.is_some())
                            .and_then(|b| b.open_path.clone());
                        if let Some(pth) = open_path {
                            #[cfg(target_os = "macos")]
                            {
                                let _ = std::process::Command::new("open").arg(&pth).spawn();
                            }
                            #[cfg(target_os = "windows")]
                            {
                                let _ = std::process::Command::new("cmd")
                                    .args(["/C", "start", "", &pth])
                                    .spawn();
                            }
                            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                            {
                                let _ = std::process::Command::new("xdg-open").arg(&pth).spawn();
                            }
                        }
                        state.sidebar_focus = false;
                    }
                    SidebarRow::LiveBlockDismiss(_) => {

                        let dismiss_id = state.sidebar.live_blocks.first().map(|b| b.id.clone());
                        if let Some(id) = dismiss_id {
                            LiveBlock::dismiss(&id);
                            state.sidebar.live_blocks.retain(|b| b.id != id);
                            state.dirty = true;
                        }
                        state.sidebar_focus = false;
                    }
                    SidebarRow::LiveBlockCopy(_) => {

                        let copy_text = state
                            .sidebar
                            .live_blocks
                            .iter()
                            .find(|b| b.copy_text.is_some())
                            .and_then(|b| b.copy_text.clone());
                        if let Some(txt) = copy_text {
                            super::nav::copy_to_clipboard(&txt);
                        }
                        state.sidebar_focus = false;
                    }
                    SidebarRow::LiveBlockResume(_) => {

                        let resume = state
                            .sidebar
                            .live_blocks
                            .iter()
                            .find(|b| b.resume_command.is_some())
                            .and_then(|b| b.resume_command.clone());
                        if let Some(cmd) = resume {
                            let session_id = state
                                .sidebar
                                .live_blocks
                                .iter()
                                .find(|b| !b.session_id.is_empty())
                                .map(|b| b.session_id.clone())
                                .unwrap_or_default();
                            let run_pane = state.panes.iter().position(|p| {
                                p.lock()
                                    .map(|g| {
                                        g.state.session_id.as_deref() == Some(session_id.as_str())
                                            || (!session_id.is_empty()
                                                && (g.state.launch_cmd.contains(
                                                    &format!("--session {session_id}"),
                                                ) || g.state.launch_cmd.contains(&format!(
                                                    "--resume {session_id}"
                                                ))))
                                    })
                                    .unwrap_or(false)
                            });
                            let target_pane = run_pane
                                .or_else(|| (state.active < state.panes.len()).then_some(state.active))
                                .unwrap_or(0);
                            if let Some(pane_arc) = state.panes.get(target_pane) {
                                if let Ok(mut p) = pane_arc.lock() {
                                    send_slash_command(&mut p, &cmd);
                                }
                            }
                            state.focus_tab(target_pane);
                        }
                        state.sidebar_focus = false;
                    }
                    SidebarRow::RightSidebar(idx) => {

                        if state.panel_sidebar_open && state.active_right_sidebar == idx {
                            state.panel_sidebar_open = false;
                            state.panel_maximized = false;
                            state.panel_focused = false;
                        } else {
                            state.active_right_sidebar = idx;
                            state.panel_sidebar_open = true;
                            state.panel_maximized = false;
                            state.refresh_mods_now();
                        }
                        state.dirty = true;
                        state.sidebar_focus = false;
                    }
                }
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
                let geoms = crate::ui::tab_bar::tab_geometries(tab_area, &titles, closable);
                let mut clicked_tab: Option<(usize, bool)> = None;
                let mut clicked_plus = false;

                for (idx, geom) in geoms.iter().enumerate() {
                    if col >= geom.start_x && col < geom.start_x + geom.width {
                        let close_zone = geom.body_x + geom.body_len.saturating_sub(3);
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
                if clicked_tab.is_none() && col >= plus_x && col < plus_x + 5 {
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

pub fn handle_scroll_accum(
    state: &mut AppState,
    mouse: MouseEvent,
    scroll_delta: i32,
) -> io::Result<()> {

    let viewport_h = state
        .panes
        .get(state.active)
        .map(|p| p.lock().map(|g| g.term.screen_lines()).unwrap_or(24))
        .unwrap_or(24) as u16;
    let delta = state.scroll_physics.apply(scroll_delta, viewport_h);

    let sidebar_w = state.sidebar_w;
    let over_sidebar = mouse.column < sidebar_w;
    let over_tab_bar = !over_sidebar && mouse.row == 0;

    if over_sidebar {
        if delta > 0 {
            state.sidebar.prev();
        } else {
            state.sidebar.next();
        }
    } else if over_tab_bar {
        if delta > 0 {
            state.active = if state.active == 0 {
                state.panes.len().saturating_sub(1)
            } else {
                state.active - 1
            };
            state.refresh_mods_now();
        } else if !state.panes.is_empty() {
            state.active = (state.active + 1) % state.panes.len();
            state.refresh_mods_now();
        }
        state.sidebar_focus = false;
    } else if let Some(pane) = state.panes.get(state.active) {
        let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());
        let mode = *p.term.mode();
        let mouse_report = mode.intersects(
            alacritty_terminal::term::TermMode::MOUSE_REPORT_CLICK
                | alacritty_terminal::term::TermMode::MOUSE_DRAG
                | alacritty_terminal::term::TermMode::MOUSE_MOTION
                | alacritty_terminal::term::TermMode::SGR_MOUSE,
        );

        if mouse_report {

            let up = scroll_delta < 0;
            let btn = if up { 64 } else { 65 };
            let (content_left, content_top) = content_origin(state, 80);
            let col = mouse.column.saturating_sub(content_left) + 1;
            let row = mouse.row.saturating_sub(content_top) + 1;
            let seq = format!("\x1b[<{btn};{col};{row}M");
            p.write_input(seq.as_bytes());
        } else if mode.contains(alacritty_terminal::term::TermMode::ALT_SCREEN) {

            let up = scroll_delta < 0;
            let seq = if up { b"\x1b[A" } else { b"\x1b[B" };
            for _ in 0..scroll_delta.abs().min(5) {
                p.write_input(seq);
            }
        } else {

            p.scroll_display(delta);
        }
    }
    Ok(())
}

pub fn execute_context_menu_action(
    state: &mut AppState,
    action: crate::ui::context_menu::ContextMenuAction,
    new_tab_cmd: &str,
) {
    use crate::ui::context_menu::ContextMenuAction;
    match action {
        ContextMenuAction::TabRename(tab_idx) => {
            if let Some(pane) = state.panes.get(tab_idx) {
                let current_title = pane.lock().unwrap_or_else(|e| e.into_inner()).state.title.clone();
                let mut m = Modal::new("rename_tab", format!("Rename Terminal #{}", tab_idx + 1));
                m.rows.push(ModalRow::TextInput {
                    key: "title".into(),
                    label: "Title".into(),
                    value: if current_title == "commandcode" { String::new() } else { current_title },
                });
                m.rows.push(ModalRow::Info("Type new name, press ENTER to save or ESC to cancel".into()));
                state.active_modal = Some(m);
            }
        }
        ContextMenuAction::TabDuplicate(_tab_idx) => {
            let (cols, rows) = active_pane_size(state);
            let mut tab_cmd = new_tab_cmd.to_string();
            if state.sidebar.yolo_mode && !tab_cmd.contains("--yolo") {
                tab_cmd.push_str(" --yolo");
            }
            if spawn_pane(state, &tab_cmd, cols, rows).is_ok() {
                state.active = state.panes.len() - 1;
                state.refresh_mods_now();
            }
        }
        ContextMenuAction::TabClose(tab_idx) => {
            if state.panes.len() > 1 {
                state.close_pane(tab_idx);
            }
        }
        ContextMenuAction::TabSplitRight(_tab_idx) => {
            let (cols, rows) = active_pane_size(state);
            let tab_cmd = new_tab_cmd.to_string();
            if spawn_pane(state, &tab_cmd, cols, rows).is_ok() {
                state.active = state.panes.len() - 1;
                state.refresh_mods_now();
            }
        }
        ContextMenuAction::TabSplitDown(_tab_idx) => {
            let (cols, rows) = active_pane_size(state);
            let tab_cmd = new_tab_cmd.to_string();
            if spawn_pane(state, &tab_cmd, cols, rows).is_ok() {
                state.active = state.panes.len() - 1;
                state.refresh_mods_now();
            }
        }
        ContextMenuAction::SessionOpen(id) => {
            let (cols, rows) = active_pane_size(state);
            let cmd = format!("{} --resume {}", new_tab_cmd, id);
            if spawn_pane(state, &cmd, cols, rows).is_ok() {
                state.active = state.panes.len() - 1;
                state.refresh_mods_now();
            }
        }
        ContextMenuAction::SessionDelete(id) => {
            if let Some(pos) = state.sidebar.sessions.iter().position(|s| s.id == id) {
                state.sidebar.delete_session(pos);
            }
            state.dirty = true;
        }
        ContextMenuAction::SessionCopyId(id) => {
            super::nav::copy_to_clipboard(&id);
        }
        ContextMenuAction::PaneInspect(pane_idx) => {
            super::input::open_cmd_inspect(state, pane_idx);
        }
        ContextMenuAction::PaneClear(pane_idx) => {
            if let Some(pane) = state.panes.get(pane_idx) {
                if let Ok(mut p) = pane.lock() {
                    p.write_input(b"\x0c");
                }
            }
        }
        ContextMenuAction::PaneScrollback(pane_idx) => {
            if let Some(pane) = state.panes.get(pane_idx) {
                if let Ok(mut p) = pane.lock() {
                    p.scroll_display(10);
                }
            }
        }
    }
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
        assert_eq!(left, 1);
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

