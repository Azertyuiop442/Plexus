use alacritty_terminal::grid::Dimensions;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::modals::{
    open_ai_prefs_modal, open_all_sessions_modal, open_all_sessions_modal_with_msg,
    open_context_modal, open_full_config_modal, open_mod_config_modal, open_navigator_modal,
    sync_modal_toggles,
};
use super::pane_ops::{active_pane_size, spawn_pane};
use crate::state::AppState;
use crate::ui::modal::{open_auto_retry_modal, Modal, ModalRow};
use crate::ui::pane::key_to_bytes;
use crate::ui::sidebar::{session_resumable, SettingsSubMenu, SidebarRow};

pub use super::mouse::{handle_mouse, handle_scroll_accum};
#[allow(unused_imports)]
pub use super::mouse::build_selected_row;
pub use super::dictation::flush_dictation;
pub use super::nav::{change_pane_cwd, read_clipboard, reload_mux, shell_quote};

fn handle_skills_modal_enter(state: &mut AppState) {
    if !crate::ui::modal::skills::is_skills_modal(state) {
        return;
    }
    let step = crate::ui::modal::skills::current_step(state);
    let modal = state.active_modal.as_ref().expect("skills modal present");
    let in_commands = modal.selected_is_command();
    let cmd_idx = modal.selected.saturating_sub(modal.rows.len());
    let cmd_name = modal.commands.get(cmd_idx).map(|(n, _)| n.clone());

    if in_commands {
        match cmd_name.as_deref() {
            Some("back") => handle_skills_back(state),
            Some("refresh") => {
                crate::skills::check_all_background(state.events.clone());
                crate::ui::modal::open_skills_modal(state);
            }
            Some("close") | _ => {
                state.active_modal = None;
            }
        }
        return;
    }

    let selected_idx = modal.selected;
    let key = modal.rows.get(selected_idx).and_then(row_key);
    let key = match key {
        Some(k) => k,
        None => return,
    };

    match step {
        0 => handle_skills_browse(state, &key),
        1 => handle_skills_tracker(state, &key),
        2 => handle_skills_sources(state, &key),
        3 => handle_skills_install(state, &key),
        _ => {}
    }
}

fn row_key(row: &crate::ui::modal::model::ModalRow) -> Option<String> {
    match row {
        crate::ui::modal::model::ModalRow::Toggle { key, .. } => Some(key.clone()),
        crate::ui::modal::model::ModalRow::Nav { key, .. } => Some(key.clone()),
        crate::ui::modal::model::ModalRow::TextInput { key, .. } => Some(key.clone()),
        crate::ui::modal::model::ModalRow::Stepper { key, .. } => Some(key.clone()),
        crate::ui::modal::model::ModalRow::Choice { key, .. } => Some(key.clone()),
        _ => None,
    }
}

fn handle_skills_back(state: &mut AppState) {
    use crate::ui::modal::skills as sm;
    let step = sm::current_step(state);
    if step == 0 && !state.skills_view.path.is_empty() {
        state.skills_view.path.pop();
        state.skills_view.selected_file = None;
        crate::ui::modal::open_skills_modal(state);
        return;
    }
    if step > 0 {
        if let Some(modal) = state.active_modal.as_mut() {
            modal.set_step(step - 1);
        }
        return;
    }
    state.active_modal = None;
}

fn handle_skills_browse(state: &mut AppState, key: &str) {
    if let Some(rest) = key.strip_prefix("vendor.open.") {
        state.skills_view.path.clear();
        state.skills_view.path.push(rest.to_string());
        state.skills_view.selected_file = None;
        crate::ui::modal::open_skills_modal(state);
        return;
    }
    if let Some(rest) = key.strip_prefix("skill.open.") {
        let mut parts = rest.splitn(2, '.');
        let vendor = parts.next().unwrap_or("").to_string();
        let skill = parts.next().unwrap_or("").to_string();
        if !vendor.is_empty() && !skill.is_empty() {
            state.skills_view.path.clear();
            state.skills_view.path.push(vendor);
            state.skills_view.path.push(skill);
            state.skills_view.selected_file = None;
            crate::ui::modal::open_skills_modal(state);
        }
        return;
    }
    if let Some(rest) = key.strip_prefix("file.open.") {
        let mut parts = rest.splitn(3, '.');
        let _vendor = parts.next().unwrap_or("");
        let _skill = parts.next().unwrap_or("");
        let file = parts.next().unwrap_or("").to_string();
        state.skills_view.selected_file = Some(file);
        write_pickup_and_close(state);
        return;
    }
}

fn handle_skills_tracker(state: &mut AppState, key: &str) {
    if key == "update_all" {
        if state.skills_view.updating.is_some() {
            return;
        }
        let targets: Vec<String> = crate::skills::vendor_statuses()
            .into_iter()
            .filter(|s| s.is_stale())
            .map(|s| s.name)
            .collect();
        if targets.is_empty() {
            return;
        }
        let total = targets.len();
        state.skills_view.updating = Some(crate::state::SkillsUpdateProgress {
            total,
            done: 0,
            current: targets[0].clone(),
            last_result: Some(format!("Queued {} vendor{}", total, if total == 1 { "" } else { "s" })),
            started_at_ms: now_ms(),
        });
        crate::ui::modal::open_skills_modal(state);
        crate::skills::update_all_behind_async(state.events.clone());
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn handle_skills_sources(state: &mut AppState, _key: &str) {
    if let Some(modal) = state.active_modal.as_mut() {
        for row in &modal.rows {
            if let crate::ui::modal::model::ModalRow::TextInput { key, value, .. } = row {
                if let Some(vendor) = key.strip_prefix("url.") {
                    let trimmed = value.trim();
                    if !trimmed.is_empty() {
                        let _ = crate::skills::attach_url(vendor, trimmed);
                    }
                }
            }
        }
        if modal.current_step < modal.steps.len() {
            modal.steps[modal.current_step].rows = modal.rows.clone();
        }
    }
    crate::skills::check_all_background(state.events.clone());
    state.dirty = true;
}

fn handle_skills_install(state: &mut AppState, key: &str) {
    if key != "install.action" && key != "install.url" && key != "install.vendor" {
        return;
    }
    let mut url = String::new();
    let mut vendor = String::new();
    if let Some(modal) = state.active_modal.as_ref() {
        for row in &modal.rows {
            if let crate::ui::modal::model::ModalRow::TextInput { key, value, .. } = row {
                if key == "install.url" {
                    url = value.trim().to_string();
                } else if key == "install.vendor" {
                    vendor = value.trim().to_string();
                }
            }
        }
    }
    if url.is_empty() {
        return;
    }
    let vendor_opt = if vendor.is_empty() { None } else { Some(vendor.as_str()) };
    match crate::skills::install_skills_bundle(&url, vendor_opt) {
        Ok((vendor_name, count)) => {
            let msg = format!("✓ Installed {count} skill{} for {vendor_name}", if count == 1 { "" } else { "s" });
            crate::ipc::log_append(
                "skills-errors.log",
                &format!("installed {count} skills for vendor {vendor_name} from {url}"),
            );
            state.skills_view.last_update_summary = Some(msg);
            crate::skills::check_all_background(state.events.clone());
            crate::ui::modal::open_skills_modal(state);
        }
        Err(e) => {
            let msg = format!("✗ Install failed: {e}");
            crate::ipc::log_append("skills-errors.log", &format!("install failed: {e}"));
            state.skills_view.last_update_summary = Some(msg);
            crate::ui::modal::open_skills_modal(state);
            if let Some(modal) = state.active_modal.as_mut() {
                modal.set_step(3);
            }
        }
    }
}

fn write_pickup_and_close(state: &mut AppState) {
    let path = match (
        state.skills_view.path.first().cloned(),
        state.skills_view.path.get(1).cloned(),
        state.skills_view.selected_file.clone(),
    ) {
        (Some(v), Some(s), Some(f)) => format!("{v}/{s}/{f}"),
        _ => return,
    };
    let session_id = state
        .panes
        .get(state.active)
        .and_then(|p| p.lock().ok())
        .and_then(|p| p.state.session_id.clone())
        .unwrap_or_default();
    let seq = state.last_pickup_seq.map(|s| s + 1).unwrap_or(1);
    state.last_pickup_seq = Some(seq);
    let pickup = serde_json::json!({
        "mod": "skills",
        "modal": "skill_file",
        "value": path,
        "sessionId": session_id,
        "seq": seq,
    });
    if let Ok(json) = serde_json::to_string_pretty(&pickup) {
        let _ = crate::ipc::atomic_write(
            &std::path::Path::new(&crate::ipc::ipc_path("mod-pickup.json")),
            &json,
        );
    }
    state.active_modal = None;
}

pub fn handle_key(state: &mut AppState, key: KeyEvent, command: &str, new_tab_cmd: &str) {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);

    let panel_active = state.panel_sidebar_open || state.panel_maximized;
    if panel_active && state.panel_focused {

        if ctrl && key.code == KeyCode::Char('p') {
            state.panel_sidebar_open = false;
            state.panel_maximized = false;
            state.panel_focused = false;
            state.dirty = true;
            return;
        }
        let panel = state
            .mods_data
            .mods
            .iter()
            .find_map(|m| m.data.panels.first().cloned());
        if let Some(panel) = panel {
            match crate::ui::mod_panel::handle_key(key, &panel, &mut state.panel_state) {
                crate::ui::mod_panel::PanelAction::Handled => {
                    state.dirty = true;
                    return;
                }
                crate::ui::mod_panel::PanelAction::Close => {
                    state.panel_sidebar_open = false;
                    state.panel_maximized = false;
                    state.panel_focused = false;
                    state.dirty = true;
                    return;
                }
                crate::ui::mod_panel::PanelAction::Maximize => {
                    state.panel_maximized = !state.panel_maximized;
                    if state.panel_maximized {
                        state.panel_sidebar_open = false;
                    } else {
                        state.panel_sidebar_open = true;
                    }
                    state.dirty = true;
                    return;
                }
                crate::ui::mod_panel::PanelAction::Pickup { row_value, .. } => {

                    let value = if row_value.is_empty() {
                        panel.default_value.clone()
                    } else {
                        row_value
                    };
                    if value.is_empty() {
                        state.dirty = true;
                        return;
                    }

                    let mod_id = if panel.mod_id.is_empty() {
                        "mod-panel"
                    } else {
                        &panel.mod_id
                    };
                    let session_id = state
                        .panes
                        .get(state.active)
                        .and_then(|p| p.lock().ok())
                        .and_then(|p| p.state.session_id.clone())
                        .unwrap_or_default();
                    let seq = state.last_pickup_seq.map(|s| s + 1).unwrap_or(1);
                    state.last_pickup_seq = Some(seq);
                    let pickup = serde_json::json!({
                        "mod": mod_id,
                        "modal": panel.id,
                        "value": value,
                        "sessionId": session_id,
                        "seq": seq,
                    });
                    if let Ok(json) = serde_json::to_string_pretty(&pickup) {
                        let _ = crate::ipc::atomic_write(
                            &std::path::Path::new(&crate::ipc::ipc_path("mod-pickup.json")),
                            &json,
                        );
                    }
                    state.dirty = true;
                    return;
                }
                crate::ui::mod_panel::PanelAction::Ignored => {

                    return;
                }
            }
        }
    }

    if state.finder.is_some() {
        match key.code {
            KeyCode::Esc => {
                state.finder = None;
                state.dirty = true;
                return;
            }
            KeyCode::Tab => {
                if let Some(ref mut finder) = state.finder {
                    finder.next_scope(&state.panes);
                }
                state.dirty = true;
                return;
            }
            KeyCode::Up => {
                if let Some(ref mut finder) = state.finder {
                    finder.move_up();
                }
                state.dirty = true;
                return;
            }
            KeyCode::Down => {
                if let Some(ref mut finder) = state.finder {
                    finder.move_down();
                }
                state.dirty = true;
                return;
            }
            KeyCode::Backspace => {
                if let Some(ref mut finder) = state.finder {
                    finder.query.pop();
                    finder.update_results(&state.panes);
                }
                state.dirty = true;
                return;
            }
            KeyCode::Enter => {
                let action = state.finder.as_ref().and_then(|f| {
                    f.results.get(f.selected).map(|item| item.kind.clone())
                });
                state.finder = None;
                state.dirty = true;
                if let Some(kind) = action {
                    match kind {
                        crate::ui::search::ItemKind::Tab { idx } => {
                            if idx < state.panes.len() {
                                state.focus_tab(idx);
                            }
                        }
                        crate::ui::search::ItemKind::File { path } => {
                            let cwd = state
                                .panes
                                .get(state.active)
                                .and_then(|p| p.lock().ok())
                                .and_then(|p| p.state.pending_cwd.clone());
                            crate::ui::links::open_file_in_editor(&path, None, cwd.as_deref());
                        }
                        crate::ui::search::ItemKind::Output { pane_idx, .. } => {
                            if pane_idx < state.panes.len() {
                                state.focus_tab(pane_idx);
                            }
                        }
                    }
                }
                return;
            }
            KeyCode::Char(ch) if !ctrl => {
                if let Some(ref mut finder) = state.finder {
                    finder.query.push(ch);
                    finder.update_results(&state.panes);
                }
                state.dirty = true;
                return;
            }
            _ => {
                return;
            }
        }
    }

    if state.context_menu.is_some() {
        state.context_menu = None;
        state.dirty = true;
        return;
    }

    if let Some(ref mut inspect) = state.cmd_inspect {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                state.cmd_inspect = None;
                state.dirty = true;
            }
            KeyCode::Char('r') => {
                let pane_idx = inspect.pane_idx;
                open_cmd_inspect(state, pane_idx);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if inspect.scroll > 0 {
                    inspect.scroll -= 1;
                    state.dirty = true;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                inspect.scroll += 1;
                state.dirty = true;
            }
            _ => {
                state.cmd_inspect = None;
                state.dirty = true;
            }
        }
        return;
    }

    if let Some(ref mut switcher) = state.switcher {
        match key.code {
            KeyCode::Esc => {
                state.switcher = None;
                state.dirty = true;
            }
            KeyCode::Up => {
                switcher.prev();
                state.dirty = true;
            }
            KeyCode::Down => {
                switcher.next();
                state.dirty = true;
            }
            KeyCode::Backspace => {
                switcher.backspace();
                state.dirty = true;
            }
            KeyCode::Char(c) if !ctrl => {
                switcher.insert_char(c);
                state.dirty = true;
            }
            KeyCode::Enter => {
                let action = switcher.selected_action();
                state.switcher = None;
                state.dirty = true;
                if let Some(act) = action {
                    execute_switcher_action(state, act, new_tab_cmd);
                }
            }
            _ => {}
        }
        return;
    }

    if state.picker.is_some() {

        const PICKER_PAGE: usize = 14;
        match key.code {
            KeyCode::Esc => {
                state.picker = None;
            }
            KeyCode::PageUp => {
                if let Some(p) = state.picker.as_mut() {
                    p.picker.page_move(-1, PICKER_PAGE);
                }
            }
            KeyCode::PageDown => {
                if let Some(p) = state.picker.as_mut() {
                    p.picker.page_move(1, PICKER_PAGE);
                }
            }

            KeyCode::Char('u') if ctrl => {
                if let Some(p) = state.picker.as_mut() {
                    p.picker.page_move(-1, PICKER_PAGE);
                }
            }
            KeyCode::Char('d') if ctrl => {
                if let Some(p) = state.picker.as_mut() {
                    p.picker.page_move(1, PICKER_PAGE);
                }
            }
            KeyCode::Char('[') => {
                if let Some(p) = state.picker.as_mut() {
                    p.picker.page_move(-1, PICKER_PAGE);
                }
            }
            KeyCode::Char(']') => {
                if let Some(p) = state.picker.as_mut() {
                    p.picker.page_move(1, PICKER_PAGE);
                }
            }
            KeyCode::Down => {
                if let Some(p) = state.picker.as_mut() {
                    p.picker.move_selection(1);
                }
            }
            KeyCode::Up => {
                if let Some(p) = state.picker.as_mut() {
                    p.picker.move_selection(-1);
                }
            }
            KeyCode::Left => {
                if let Some(p) = state.picker.as_mut() {
                    let cat = (p.picker.category + 3) % 4;
                    p.picker.set_category(cat);
                }
            }
            KeyCode::Right | KeyCode::Tab => {
                if let Some(p) = state.picker.as_mut() {
                    p.picker.cycle_category();
                }
            }
            KeyCode::Backspace => {
                if let Some(p) = state.picker.as_mut() {
                    p.picker.query.pop();
                    p.picker.selected = 0;
                }
            }
            KeyCode::Char(ch) => {
                if let Some(p) = state.picker.as_mut() {
                    p.picker.query.push(ch);
                    p.picker.selected = 0;
                }
            }
            KeyCode::Enter => {
                let selected_opt = state
                    .picker
                    .as_ref()
                    .and_then(|p| p.picker.highlighted_option());
                let row_idx = state.picker.as_ref().map(|p| p.row_idx);
                if let (Some(opt_idx), Some(target_row)) = (selected_opt, row_idx) {
                    if let Some(ref mut modal) = state.active_modal {
                        modal.selected = target_row;
                        modal.select_option(opt_idx);
                        modal.dirty = true;
                        modal.save();
                    }
                }
                state.picker = None;
            }
            _ => {}
        }
        return;
    }

    if let Some(ref mut modal) = state.active_modal {
        let is_all_sessions = modal.id == "all_sessions";

        let selected_session_id = || {
            let row = modal.rows.get(modal.selected)?;
            match row {
                ModalRow::Choice { key, options, current, .. } => {
                    let id = options
                        .get(*current)
                        .and_then(|(_, v, _)| v.strip_prefix("open_").or_else(|| v.strip_prefix("del_")))
                        .map(str::to_string)
                        .or_else(|| key.strip_prefix("sess_").map(str::to_string));
                    id.filter(|id| !id.is_empty())
                }
                _ => None,
            }
        };

        match key.code {
            KeyCode::Esc => {
                if modal.editing_text {

                    modal.editing_text = false;
                    return;
                }
                if modal.id == "confirm_close" {
                    state.confirm_close_idx = None;
                }
                sync_modal_toggles(state);
                state.active_modal = None;
            }
            KeyCode::PageUp => {
                modal.page_move(-1);
            }
            KeyCode::PageDown => {
                modal.page_move(1);
            }

            KeyCode::Char('u') if ctrl && modal.page_size > 0 => {
                modal.page_move(-1);
            }
            KeyCode::Char('d') if ctrl && modal.page_size > 0 => {
                modal.page_move(1);
            }
            KeyCode::Char('[') if modal.page_size > 0 => {
                modal.page_move(-1);
            }
            KeyCode::Char(']') if modal.page_size > 0 => {
                modal.page_move(1);
            }
            KeyCode::Delete | KeyCode::Char('d') if modal.id.starts_with("list_") => {
                let mod_id = modal.id.trim_start_matches("list_").to_string();
                let modal_title = modal.title.clone();
                let value = modal.run_files.get(modal.selected).cloned();
                if let Some(value) = value {
                    let session_id = state
                        .panes
                        .get(state.active)
                        .and_then(|p| p.lock().ok())
                        .and_then(|p| p.state.session_id.clone())
                        .unwrap_or_default();
                    let seq = state
                        .last_pickup_seq
                        .map(|s| s + 1)
                        .unwrap_or(1);
                    state.last_pickup_seq = Some(seq);
                    let pickup = serde_json::json!({
                        "mod": mod_id,
                        "modal": modal_title,
                        "value": value,
                        "action": "delete",
                        "key": "d",
                        "sessionId": session_id,
                        "seq": seq,
                    });
                    if let Ok(json) = serde_json::to_string_pretty(&pickup) {
                        let _ = crate::ipc::atomic_write(
                            &std::path::Path::new(&crate::ipc::ipc_path("mod-pickup.json")),
                            &json,
                        );
                    }
                    if modal.selected < modal.rows.len() {
                        modal.rows.remove(modal.selected);
                        if modal.selected < modal.run_files.len() {
                            modal.run_files.remove(modal.selected);
                        }
                        if modal.rows.is_empty() {
                            state.active_modal = None;
                        } else {
                            if modal.selected >= modal.rows.len() {
                                modal.selected = modal.rows.len().saturating_sub(1);
                            }
                            modal.dirty = true;
                        }
                    }
                }
            }
            KeyCode::Delete | KeyCode::Char('d') if is_all_sessions => {

                let Some(session_id) = selected_session_id() else {
                    return;
                };
                let Some(sel_idx) = state
                    .sidebar
                    .sessions
                    .iter()
                    .position(|s| s.id == session_id)
                else {

                    open_all_sessions_modal(state);
                    return;
                };
                if sel_idx < state.sidebar.sessions.len() {

                    let is_open = state
                        .sidebar
                        .sessions
                        .get(sel_idx)
                        .map(|s| state.session_is_open(&s.id))
                        .unwrap_or(false);
                    if !is_open {
                        state.sidebar.delete_session(sel_idx);
                        open_all_sessions_modal(state);
                    }
                }
            }
            KeyCode::Delete | KeyCode::Char('d') if modal.id == "navigator" => {
                let idx = modal.selected;
                if idx < state.panes.len() {
                    state.close_pane(idx);
                    open_navigator_modal(state);
                }
            }
            KeyCode::Enter => {
                if modal.id == "rename_tab" {
                    if let Some(ModalRow::TextInput { value, .. }) = modal.rows.first() {
                        if let Some(pane) = state.panes.get(state.active) {
                            pane.lock().unwrap_or_else(|e| e.into_inner()).state.title = value.trim().to_string();
                        }
                    }
                    state.active_modal = None;
                } else if modal.id == "confirm_close" {

                    let do_close = matches!(
                        modal.rows.get(modal.selected),
                        Some(ModalRow::Choice { current: 0, .. })
                    );
                    let idx = state.confirm_close_idx.take();
                    state.active_modal = None;
                    if do_close {
                        if let Some(idx) = idx {
                            state.close_pane(idx);
                        }
                    }
                } else if modal.id == "skills_config" {
                    handle_skills_modal_enter(state);
                    return;
                } else if modal.id.starts_with("list_") {

                    let mod_id = modal.id.trim_start_matches("list_").to_string();
                    let modal_title = modal.title.clone();
                    let value = modal.run_files.get(modal.selected).cloned();
                    let pickup_cmd = modal.pickup_command.clone();
                    state.active_modal = None;
                    if let Some(value) = value {
                        let session_id = state
                            .panes
                            .get(state.active)
                            .and_then(|p| p.lock().ok())
                            .and_then(|p| p.state.session_id.clone())
                            .unwrap_or_default();
                        let seq = state
                            .last_pickup_seq
                            .map(|s| s + 1)
                            .unwrap_or(1);
                        state.last_pickup_seq = Some(seq);
                        let pickup = serde_json::json!({
                            "mod": mod_id,
                            "modal": modal_title,
                            "value": value,
                            "sessionId": session_id,
                            "seq": seq,
                        });
                        if let Ok(json) = serde_json::to_string_pretty(&pickup) {
                            let _ = crate::ipc::atomic_write(
                                &std::path::Path::new(&crate::ipc::ipc_path("mod-pickup.json")),
                                &json,
                            );
                        }
                        if let Some(pane) = state.panes.get(state.active) {
                            if let Ok(mut p) = pane.lock() {

                                if !pickup_cmd.is_empty() {
                                    super::nav::send_slash_command(&mut p, &pickup_cmd);
                                }
                            }
                        }
                    }
                } else if modal.id == "navigator" {

                    let idx = modal.selected;
                    if idx < state.panes.len() {
                        state.focus_tab(idx);
                    }
                    state.active_modal = None;
                } else if is_all_sessions {
                    if let Some(ModalRow::Choice { options, current, .. }) = modal.rows.get(modal.selected) {
                        let opt_val = options.get(*current).map(|(_, v, _)| v.as_str()).unwrap_or("");
                        let open_ids: Vec<String> = state
                            .panes
                            .iter()
                            .filter_map(|p| p.lock().ok())
                            .filter_map(|p| p.state.session_id.clone())
                            .filter(|id| !id.is_empty())
                            .collect();
                        if opt_val == "execute_clean_all" {
                            let count = state.sidebar.clean_all_sessions(&open_ids);
                            let msg = format!("✓ Cleaned {count} workspace session(s)");
                            open_all_sessions_modal_with_msg(state, Some(&msg));
                            if let Some(ref mut m) = state.active_modal {
                                m.set_step(1);
                            }
                            return;
                        } else if opt_val == "execute_clean_24h" {
                            let count = state.sidebar.clean_old_hours(24, &open_ids);
                            let msg = if count > 0 {
                                format!("✓ Cleaned {count} session(s) older than 24h")
                            } else {
                                "ℹ No sessions were older than 24h".to_string()
                            };
                            open_all_sessions_modal_with_msg(state, Some(&msg));
                            if let Some(ref mut m) = state.active_modal {
                                m.set_step(1);
                            }
                            return;
                        } else if opt_val == "execute_clean_3d" {
                            let count = state.sidebar.clean_old_sessions(3, &open_ids);
                            let msg = if count > 0 {
                                format!("✓ Cleaned {count} session(s) older than 3 days")
                            } else {
                                "ℹ No sessions were older than 3 days".to_string()
                            };
                            open_all_sessions_modal_with_msg(state, Some(&msg));
                            if let Some(ref mut m) = state.active_modal {
                                m.set_step(1);
                            }
                            return;
                        } else if opt_val == "execute_clean_7d" {
                            let count = state.sidebar.clean_old_sessions(7, &open_ids);
                            let msg = if count > 0 {
                                format!("✓ Cleaned {count} session(s) older than 7 days")
                            } else {
                                "ℹ No sessions were older than 7 days".to_string()
                            };
                            open_all_sessions_modal_with_msg(state, Some(&msg));
                            if let Some(ref mut m) = state.active_modal {
                                m.set_step(1);
                            }
                            return;
                        }
                    }

                    let Some(session_id) = selected_session_id() else {
                        state.active_modal = None;
                        return;
                    };
                    let Some(sel_idx) = state
                        .sidebar
                        .sessions
                        .iter()
                        .position(|s| s.id == session_id)
                    else {

                        state.active_modal = None;
                        return;
                    };
                    if let Some(ModalRow::Choice {
                        options, current, ..
                    }) = modal.rows.get(modal.selected)
                    {
                        let is_delete = options
                            .get(*current)
                            .map(|(_, v, _)| v.starts_with("del_"))
                            .unwrap_or(false);
                        if is_delete {

                            let is_open = state
                                .sidebar
                                .sessions
                                .get(sel_idx)
                                .map(|s| state.session_is_open(&s.id))
                                .unwrap_or(false);
                            if !is_open && sel_idx < state.sidebar.sessions.len() {
                                state.sidebar.delete_session(sel_idx);
                                open_all_sessions_modal(state);
                            }
                        } else {
                            if let Some(session) = state.sidebar.sessions.get(sel_idx) {
                                let session_id = session.id.clone();

                                let already_open = if state.session_is_open(&session_id) {
                                    state.panes.iter().position(|p| {
                                        p.lock()
                                            .map(|g| g.is_session_live(&session_id))
                                            .unwrap_or(false)
                                    })
                                } else {
                                    None
                                };
                                if let Some(pane_idx) = already_open {
                                    state.focus_tab(pane_idx);
                                    state.sidebar_focus = false;
                                } else if !session_resumable(&session_id, &state.sidebar.project) {

                                    state.sidebar.remove_session_by_id(&session_id);
                                    open_all_sessions_modal(state);
                                } else {

                                    let cwd = state.sidebar.project_cwd.clone();
                                    let mut cmd =
                                        super::nav::session_launch_cmd(&cwd, &session_id)
                                            .unwrap_or_else(|| command.to_string());

                                    if state.sidebar.yolo_mode && !cmd.contains("--yolo") {
                                        cmd.push_str(" --yolo");
                                    }
                                    let (cols, rows) = active_pane_size(state);
                                    if spawn_pane(state, &cmd, cols, rows).is_ok() {
                                        state.active = state.panes.len() - 1;
                                        state.sidebar_focus = false;
                                    }
                                }
                            }
                            state.active_modal = None;
                        }
                    } else {
                        state.active_modal = None;
                    }
                } else if modal.selected_is_command() {
                    let cmd_idx = modal.selected.saturating_sub(modal.rows.len());
                    if let Some((name, _desc)) = modal.commands.get(cmd_idx) {
                        if let Some(pane) = state.panes.get(state.active) {

                            let mut bytes = vec![0x15u8];
                            bytes.extend_from_slice(format!("/{}\r", name).as_bytes());
                            pane.lock().unwrap_or_else(|e| e.into_inner()).write_input(&bytes);
                        }
                    }
                    sync_modal_toggles(state);
                    state.active_modal = None;
                } else if modal.selected_is_searchable_choice() {
                    let idx = modal.selected;
                    if let Some(ModalRow::Choice {
                        options, current, ..
                    }) = modal.rows.get(idx)
                    {
                        let current_value = options
                            .get(*current)
                            .map(|(_, v, _)| v.clone())
                            .unwrap_or_default();
                        let picker =
                            crate::ui::modal::ModelPicker::new(options.clone(), current_value);
                        state.picker = Some(crate::state::PickerState {
                            row_idx: idx,
                            picker,
                        });
                    }
                } else {
                    let idx = modal.selected;
                    let is_action_row = matches!(
                        modal.rows.get(idx),
                        Some(ModalRow::Toggle { .. })
                            | Some(ModalRow::Choice { .. })
                            | Some(ModalRow::Stepper { .. })
                    );
                    if is_action_row {
                        modal.cycle_selected();
                        sync_modal_toggles(state);
                    } else if matches!(modal.rows.get(idx), Some(ModalRow::TextInput { .. })) {

                        modal.editing_text = true;
                    } else {
                        sync_modal_toggles(state);
                        state.active_modal = None;
                    }
                }
            }
            KeyCode::Backspace => {
                if is_all_sessions {
                    let Some(session_id) = selected_session_id() else {
                        return;
                    };
                    let Some(sel_idx) = state
                        .sidebar
                        .sessions
                        .iter()
                        .position(|s| s.id == session_id)
                    else {
                        open_all_sessions_modal(state);
                        return;
                    };
                    if sel_idx < state.sidebar.sessions.len() {
                        state.sidebar.delete_session(sel_idx);
                        open_all_sessions_modal(state);
                    }
                } else {
                    let idx = modal.selected.min(modal.rows.len().saturating_sub(1));
                    if let Some(ModalRow::TextInput { value, .. }) = modal.rows.get_mut(idx) {
                        value.pop();
                        modal.dirty = true;
                        if !modal.steps.is_empty() && modal.current_step < modal.steps.len() {
                            modal.steps[modal.current_step].rows = modal.rows.clone();
                        }
                    } else if let Some(ModalRow::TextInput { value, .. }) = modal.rows.first_mut() {
                        value.pop();
                        modal.dirty = true;
                        if !modal.steps.is_empty() && modal.current_step < modal.steps.len() {
                            modal.steps[modal.current_step].rows = modal.rows.clone();
                        }
                    }
                }
            }
            KeyCode::Char(c) => {
                if ctrl && (c == 'c' || c == 'u') {
                    let idx = modal.selected.min(modal.rows.len().saturating_sub(1));
                    if let Some(ModalRow::TextInput { value, .. }) = modal.rows.get_mut(idx) {
                        value.clear();
                        modal.dirty = true;
                        if !modal.steps.is_empty() && modal.current_step < modal.steps.len() {
                            modal.steps[modal.current_step].rows = modal.rows.clone();
                        }
                    } else if let Some(ModalRow::TextInput { value, .. }) = modal.rows.first_mut() {
                        value.clear();
                        modal.dirty = true;
                        if !modal.steps.is_empty() && modal.current_step < modal.steps.len() {
                            modal.steps[modal.current_step].rows = modal.rows.clone();
                        }
                    } else if c == 'c' {
                        state.active_modal = None;
                    }
                    return;
                }
                if ctrl && (c == 'v' || c == 'V') {
                    if let Some(text) = crate::mux_core::nav::read_clipboard() {
                        handle_paste(state, &text);
                    }
                    return;
                }
                let idx = modal.selected.min(modal.rows.len().saturating_sub(1));
                if let Some(ModalRow::TextInput { value, .. }) = modal.rows.get_mut(idx) {
                    value.push(c);
                    modal.dirty = true;
                    if !modal.steps.is_empty() && modal.current_step < modal.steps.len() {
                        modal.steps[modal.current_step].rows = modal.rows.clone();
                    }
                } else if modal.editing_text {
                    if let Some(ModalRow::TextInput { value, .. }) = modal.rows.first_mut() {
                        value.push(c);
                        modal.dirty = true;
                        if !modal.steps.is_empty() && modal.current_step < modal.steps.len() {
                            modal.steps[modal.current_step].rows = modal.rows.clone();
                        }
                    }
                } else {
                    match c {
                        'k' => modal.move_selection(-1),
                        'j' => modal.move_selection(1),
                        ' ' => {
                            if !modal.selected_is_command() {
                                modal.cycle_selected();
                                sync_modal_toggles(state);
                            }
                        }
                        _ => {}
                    }
                }
            }
            KeyCode::Up => {
                modal.move_selection(-1);
            }
            KeyCode::Down => {
                modal.move_selection(1);
            }
            KeyCode::Tab | KeyCode::Right => {
                if modal.adjust_stepper(1) {
                    sync_modal_toggles(state);
                    return;
                }
                if modal.next_step() {
                    return;
                }
            }
            KeyCode::BackTab | KeyCode::Left => {
                if modal.adjust_stepper(-1) {
                    sync_modal_toggles(state);
                    return;
                }
                if modal.prev_step() {
                    return;
                }
            }
            _ => {}
        }
        return;
    }

    if ctrl {
        match key.code {

            KeyCode::Char('k') => {
                let cwd = state
                    .panes
                    .get(state.active)
                    .and_then(|p| p.lock().ok())
                    .and_then(|p| p.state.pending_cwd.clone());
                let mut finder = crate::ui::search::FinderState::new(cwd.as_deref());
                finder.update_results(&state.panes);
                state.finder = Some(finder);
                state.dirty = true;
                return;
            }

            KeyCode::Char('d') => {
                state.panel_sidebar_open = !state.panel_sidebar_open;
                if state.panel_sidebar_open {
                    state.panel_maximized = false;
                    state.refresh_mods_now();
                }
                state.dirty = true;
                return;
            }

            KeyCode::Char('m') => {
                if state.panel_sidebar_open || state.panel_maximized {
                    state.panel_maximized = !state.panel_maximized;
                    if state.panel_maximized {
                        state.panel_sidebar_open = false;
                    } else {
                        state.panel_sidebar_open = true;
                    }
                    state.dirty = true;
                }
                return;
            }
            KeyCode::Char('i') => {
                open_context_modal(state);
                return;
            }

            KeyCode::Char(' ') => {
                let active = state.active;
                if let Some(pane) = state.panes.get(active) {
                    let title = pane.lock().unwrap_or_else(|e| e.into_inner()).state.title.clone();
                    let display_title = if title.is_empty() || title == "commandcode" {
                        format!("Terminal {}", active + 1)
                    } else {
                        title
                    };
                    state.context_menu = Some(crate::ui::context_menu::ContextMenu::for_tab(
                        active,
                        &display_title,
                        (state.sidebar_w + 4, 1),
                    ));
                    state.dirty = true;
                }
                return;
            }

            KeyCode::Char('p') => {
                open_quick_switcher(state);
                return;
            }

            KeyCode::Char('o') => {
                open_cmd_inspect(state, state.active);
                return;
            }

            KeyCode::Char('n') => {
                crate::mux_core::modals::open_navigator_modal(state);
                return;
            }

            KeyCode::Char('g') => {
                change_pane_cwd(state);
                return;
            }
            KeyCode::Char('e') => {
                if let Some(pane) = state.panes.get(state.active) {
                    let current_title = pane.lock().unwrap_or_else(|e| e.into_inner()).state.title.clone();
                    let mut m = Modal::new(
                        "rename_tab",
                        format!("Rename Terminal #{}", state.active + 1),
                    );
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
                }
                return;
            }
            KeyCode::Left => {

                if state.panel_sidebar_open || state.panel_maximized {
                    state.panel_sidebar_w = state
                        .panel_sidebar_w
                        .saturating_sub(4)
                        .max(crate::state::PANEL_SIDEBAR_MIN);
                } else {
                    state.sidebar_w = state.sidebar_w.saturating_sub(2).max(18);
                }
                state.dirty = true;
                return;
            }
            KeyCode::Right => {
                if state.panel_sidebar_open || state.panel_maximized {
                    state.panel_sidebar_w = state
                        .panel_sidebar_w
                        .saturating_add(4)
                        .min(crate::state::PANEL_SIDEBAR_MAX);
                } else {
                    state.sidebar_w = (state.sidebar_w + 2).min(50);
                }
                state.dirty = true;
                return;
            }
            KeyCode::Char('r') => {
                reload_mux();
            }
            KeyCode::Char('t') => {
                let (cols, rows) = active_pane_size(state);
                let mut tab_cmd = new_tab_cmd.to_string();
                if state.sidebar.yolo_mode && !tab_cmd.contains("--yolo") {
                    tab_cmd.push_str(" --yolo");
                }
                if spawn_pane(state, &tab_cmd, cols, rows).is_ok() {
                    state.active = state.panes.len() - 1;
                    state.refresh_mods_now();
                }
                return;
            }
            KeyCode::Char('w') => {
                if state.panes.len() > 1 {
                    state.close_pane(state.active);
                }
                return;
            }
            KeyCode::Tab => {
                if state.panes.is_empty() {
                    return;
                }
                if shift {
                    let prev = if state.active == 0 {
                        state.panes.len() - 1
                    } else {
                        state.active - 1
                    };
                    state.focus_tab(prev);
                } else {
                    let next = (state.active + 1) % state.panes.len();
                    state.focus_tab(next);
                }
                return;
            }
            KeyCode::Char('b') => {
                state.sidebar_open = !state.sidebar_open;
                state.sidebar_focus = state.sidebar_open;
                if state.sidebar_focus {
                    state.sidebar.refresh();
                }
                return;
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                let idx = c.to_digit(10).unwrap_or(1) as usize - 1;
                if idx < state.panes.len() {
                    state.active = idx;
                }
                return;
            }
            _ => {}
        }
    }

    if !(state.sidebar_focus && state.sidebar_open) {
        match key.code {
            KeyCode::PageUp | KeyCode::PageDown => {
                if let Some(pane) = state.panes.get(state.active) {
                    let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());
                    let page = p.term.screen_lines().max(1) as i32;
                    let delta = if key.code == KeyCode::PageUp {
                        page
                    } else {
                        -page
                    };
                    p.scroll_display(delta);
                }
                return;
            }

            KeyCode::Char('?')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::SUPER) =>
            {
                crate::mux_core::modals::open_keybind_help_modal(state);
                return;
            }

            KeyCode::Char('p')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::SUPER) =>
            {
                crate::mux_core::modals::open_keybind_help_modal(state);
                return;
            }
            _ => {}
        }
    }

    if state.sidebar_focus && state.sidebar_open {
        match key.code {
            KeyCode::Tab | KeyCode::BackTab => {
                let next_tab = (state.sidebar.active_tab + 1) % 2;
                state.sidebar.set_active_tab(next_tab);
                return;
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if state.sidebar.selected_row() == Some(SidebarRow::UsageCarousel) {
                    state.sidebar.prev_usage_tab();
                    return;
                }
                if state.sidebar.active_tab == 1
                    && state.sidebar.settings_menu != SettingsSubMenu::Main
                {
                    state.sidebar.open_submenu(SettingsSubMenu::Main);
                } else {
                    state.sidebar.set_active_tab(0);
                }
                return;
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if state.sidebar.selected_row() == Some(SidebarRow::UsageCarousel) {
                    state.sidebar.next_usage_tab();
                    return;
                }
                state.sidebar.set_active_tab(1);
                return;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                state.sidebar.next();
                return;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                state.sidebar.prev();
                return;
            }
            KeyCode::Delete | KeyCode::Backspace | KeyCode::Char('d') => {
                if let Some(SidebarRow::Session(i)) = state.sidebar.selected_row() {
                    state.sidebar.delete_session(i);
                    return;
                }
            }
            KeyCode::Enter => match state.sidebar.selected_row() {
                Some(SidebarRow::UsageCarousel) => {
                    state.sidebar.next_usage_tab();
                    return;
                }
                Some(SidebarRow::NewSession) => {
                    let (cols, rows) = active_pane_size(state);

                    let mut tab_cmd = new_tab_cmd.to_string();
                    if state.sidebar.yolo_mode && !tab_cmd.contains("--yolo") {
                        tab_cmd.push_str(" --yolo");
                    }
                    if spawn_pane(state, &tab_cmd, cols, rows).is_ok() {
                        state.active = state.panes.len() - 1;
                        state.sidebar_focus = false;
                    }
                    return;
                }
                Some(SidebarRow::Session(i)) => {

                    let already_open = state.sidebar.sessions.get(i).and_then(|s| {
                        if !state.session_is_open(&s.id) {
                            return None;
                        }
                        state.panes.iter().position(|p| {
                            p.lock()
                                .map(|g| g.is_session_live(&s.id))
                                .unwrap_or(false)
                        })
                    });
                    if let Some(pane_idx) = already_open {
                        state.focus_tab(pane_idx);
                        state.sidebar_focus = false;
                        return;
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
                    return;
                }
                Some(SidebarRow::MoreSessions) => {
                    open_all_sessions_modal(state);
                    return;
                }
                Some(SidebarRow::NavPreferences) => {
                    state.sidebar.open_submenu(SettingsSubMenu::Preferences);
                    return;
                }
                Some(SidebarRow::NavModConfig) => {
                    state.sidebar.open_submenu(SettingsSubMenu::ModConfig);
                    return;
                }
                Some(SidebarRow::NavAIPrefs) => {
                    open_ai_prefs_modal(state);
                    return;
                }
                Some(SidebarRow::NavBack) => {
                    state.sidebar.open_submenu(SettingsSubMenu::Main);
                    return;
                }
                Some(SidebarRow::PrefYolo) => {
                    state.sidebar.yolo_mode = !state.sidebar.yolo_mode;
                    state.sidebar_focus = false;
                    return;
                }
                Some(SidebarRow::PrefShowUsage) => {
                    state.sidebar.show_usage = !state.sidebar.show_usage;
                    state.sidebar.rebuild_rows();
                    state.dirty = true;
                    return;
                }
                Some(SidebarRow::PrefFullConfig) => {
                    open_full_config_modal(state);
                    return;
                }
                Some(SidebarRow::PrefAutoRetry) => {
                    open_auto_retry_modal(state);
                    return;
                }
                Some(SidebarRow::PrefSkills) => {
                    crate::ui::modal::open_skills_modal(state);
                    return;
                }
                Some(SidebarRow::PrefSkillInjection) => {
                    let mut prefs = crate::prefs::Prefs::load();
                    prefs.skill_injection = !prefs.skill_injection;
                    prefs.skills.injection_enabled = prefs.skill_injection;
                    let _ = prefs.save();
                    state.sidebar.skill_injection = prefs.skill_injection;
                    state.dirty = true;
                    return;
                }
                Some(SidebarRow::ModConfig(idx)) => {
                    open_mod_config_modal(state, idx);
                    return;
                }
                Some(SidebarRow::Reload) => {
                    reload_mux();
                }
                Some(SidebarRow::LiveBlockOpen(_))
                | Some(SidebarRow::LiveBlockDismiss(_))
                | Some(SidebarRow::LiveBlockResume(_))
                | Some(SidebarRow::LiveBlockCopy(_)) => {

                }
                Some(SidebarRow::RightSidebar(idx)) => {

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
                    return;
                }
                #[allow(unused_variables)]
                _ => {}
            },
            KeyCode::Esc => {
                if state.sidebar.active_tab == 1
                    && state.sidebar.settings_menu != SettingsSubMenu::Main
                {
                    state.sidebar.open_submenu(SettingsSubMenu::Main);
                } else if state.sidebar.expanded {
                    state.sidebar.expanded = false;
                    state.sidebar.rebuild_rows();
                    state.sidebar.scroll = 0;
                } else {
                    state.sidebar_focus = false;
                }
                return;
            }
            KeyCode::Char(' ') => match state.sidebar.selected_row() {
                Some(SidebarRow::NavPreferences) => {
                    state.sidebar.open_submenu(SettingsSubMenu::Preferences);
                }
                Some(SidebarRow::NavModConfig) => {
                    state.sidebar.open_submenu(SettingsSubMenu::ModConfig);
                }
                Some(SidebarRow::NavAIPrefs) => {
                    open_ai_prefs_modal(state);
                }
                Some(SidebarRow::NavBack) => {
                    state.sidebar.open_submenu(SettingsSubMenu::Main);
                }
                Some(SidebarRow::PrefSkillInjection) => {
                    let mut prefs = crate::prefs::Prefs::load();
                    prefs.skill_injection = !prefs.skill_injection;
                    prefs.skills.injection_enabled = prefs.skill_injection;
                    let _ = prefs.save();
                    state.sidebar.skill_injection = prefs.skill_injection;
                    state.dirty = true;
                }
                Some(SidebarRow::PrefYolo) => {
                    state.sidebar.yolo_mode = !state.sidebar.yolo_mode;
                    state.sidebar_focus = false;
                }
                Some(SidebarRow::PrefShowUsage) => {
                    state.sidebar.show_usage = !state.sidebar.show_usage;
                    state.sidebar.rebuild_rows();
                    state.dirty = true;
                }
                Some(SidebarRow::ModConfig(idx)) => {
                    open_mod_config_modal(state, idx);
                }
                Some(SidebarRow::Reload) => {
                    reload_mux();
                }
                _ => {}
            },
            _ => {

                if let KeyCode::Char(c) = key.code {
                    if c.is_ascii_graphic() || c == ' ' {
                        let bytes = key_to_bytes(key);
                        if let Some(pane) = state.panes.get(state.active) {
                            pane.lock().unwrap_or_else(|e| e.into_inner()).write_input(&bytes);
                        }
                        state.sidebar_focus = false;
                        return;
                    }
                }
                return;
            }
        }
    }

    let bytes = key_to_bytes(key);

    let is_plain_char = super::dictation::is_plain_char_key(&key);
    if is_plain_char && super::dictation::maybe_append_burst(state, &bytes) {

        return;
    }

    let flush_buf = super::dictation::take_burst(state).filter(|b| !b.is_empty());
    let buffered = super::dictation::start_or_flush_burst(state, &bytes, is_plain_char);
    if buffered {

        return;
    }
    if let Some(pane) = state.panes.get(state.active) {
        let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(buf) = flush_buf {
            p.note_paste(&buf);
            p.write_input(buf.as_bytes());
        }
        p.write_input(&bytes);
    }
}

pub fn open_quick_switcher(state: &mut AppState) {
    use crate::ui::switcher::{SwitcherAction, SwitcherItem, SwitcherState};
    let mut items = Vec::new();

    for (i, p_lock) in state.panes.iter().enumerate() {
        let p = p_lock.lock().unwrap_or_else(|e| e.into_inner());
        let title = if p.state.title.is_empty() || p.state.title == "commandcode" {
            format!("Terminal {}", i + 1)
        } else {
            p.state.title.clone()
        };
        items.push(SwitcherItem {
            title: format!("{}. {}", i + 1, title),
            subtitle: format!("Jump to active tab #{}", i + 1),
            icon: "⧉",
            action: SwitcherAction::SwitchTab(i),
        });
    }

    for s in &state.sidebar.sessions {
        items.push(SwitcherItem {
            title: s.title.clone(),
            subtitle: format!("Session ({}) · {}", s.age_short, s.id),
            icon: "●",
            action: SwitcherAction::ResumeSession(s.id.clone()),
        });
    }

    items.push(SwitcherItem {
        title: "Example Pipeline".into(),
        subtitle: "Multi-Model MoA orchestration".into(),
        icon: "⚡",
        action: SwitcherAction::ExecuteSlashCommand("example".into()),
    });
    items.push(SwitcherItem {
        title: "Global Config & Preferences".into(),
        subtitle: "Provider, default model, theme".into(),
        icon: "⚙",
        action: SwitcherAction::OpenPreferences,
    });
    items.push(SwitcherItem {
        title: "Terminal Context & Cost".into(),
        subtitle: "Token usage and turn costs".into(),
        icon: "ⓘ",
        action: SwitcherAction::OpenContext,
    });

    state.switcher = Some(SwitcherState::new(items));
    state.dirty = true;
}

pub fn open_cmd_inspect(state: &mut AppState, pane_idx: usize) {
    if let Some(pane) = state.panes.get(pane_idx) {
        let p = pane.lock().unwrap_or_else(|e| e.into_inner());
        let root_pid = p.child.process_id().unwrap_or(0);
        let title = if p.state.title.is_empty() || p.state.title == "commandcode" {
            format!("Terminal {}", pane_idx + 1)
        } else {
            p.state.title.clone()
        };
        let cwd = p
            .state
            .boot_info
            .as_ref()
            .and_then(|b| b.cwd.clone())
            .unwrap_or_default();
        state.cmd_inspect = Some(crate::ui::cmdinfo::CmdInspectState::collect_for_pid(
            pane_idx, title, cwd, root_pid,
        ));
        state.dirty = true;
    }
}

pub fn execute_switcher_action(
    state: &mut AppState,
    action: crate::ui::switcher::SwitcherAction,
    new_tab_cmd: &str,
) {
    use crate::ui::switcher::SwitcherAction;
    match action {
        SwitcherAction::SwitchTab(idx) => {
            if idx < state.panes.len() {
                state.focus_tab(idx);
            }
        }
        SwitcherAction::ResumeSession(id) => {
            let (cols, rows) = active_pane_size(state);
            let cmd = format!("{} --resume {}", new_tab_cmd, id);
            if spawn_pane(state, &cmd, cols, rows).is_ok() {
                state.active = state.panes.len() - 1;
                state.refresh_mods_now();
            }
        }
        SwitcherAction::ExecuteSlashCommand(cmd) => {
            if let Some(pane) = state.panes.get(state.active) {
                if let Ok(mut p) = pane.lock() {
                    super::nav::send_slash_command(&mut p, &cmd);
                }
            }
        }
        SwitcherAction::OpenPreferences => {
            crate::mux_core::modals::open_full_config_modal(state);
        }
        SwitcherAction::OpenModConfig(idx) => {
            crate::mux_core::modals::open_mod_config_modal(state, idx);
        }
        SwitcherAction::OpenContext => {
            crate::mux_core::modals::open_context_modal(state);
        }
    }
}

pub fn handle_paste(state: &mut AppState, text: &str) {
    let clean = text.replace('\r', "").replace('\n', "");
    if let Some(modal) = state.active_modal.as_mut() {
        let idx = modal.selected.min(modal.rows.len().saturating_sub(1));
        if let Some(ModalRow::TextInput { value, .. }) = modal.rows.get_mut(idx) {
            value.push_str(&clean);
            modal.dirty = true;
            if !modal.steps.is_empty() && modal.current_step < modal.steps.len() {
                modal.steps[modal.current_step].rows = modal.rows.clone();
            }
            return;
        }
        for row in modal.rows.iter_mut() {
            if let ModalRow::TextInput { value, .. } = row {
                value.push_str(&clean);
                modal.dirty = true;
                if !modal.steps.is_empty() && modal.current_step < modal.steps.len() {
                    modal.steps[modal.current_step].rows = modal.rows.clone();
                }
                return;
            }
        }
        return;
    }
    if let Some(finder) = state.finder.as_mut() {
        finder.query.push_str(&clean);
    }
    if state.finder.is_some() {
        let panes = state.panes.clone();
        if let Some(finder) = state.finder.as_mut() {
            finder.update_results(&panes);
        }
        return;
    }
    if let Some(pane) = state.panes.get(state.active) {
        if let Ok(mut p) = pane.lock() {
            p.write_paste(text);
        }
    }
}

