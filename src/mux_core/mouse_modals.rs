
use ratatui::layout::Rect;
use crate::state::AppState;

pub fn handle_panel_click(
    state: &mut AppState,
    col: u16,
    row: u16,
    area: Rect,
) -> bool {
    if !state.panel_sidebar_open && !state.panel_maximized {
        return false;
    }

    let left = if state.sidebar_open && area.width > state.sidebar_w + 8 {
        state.sidebar_w
    } else if area.width > 2 {
        1
    } else {
        0
    };
    let (panel_area, _) = if state.panel_maximized {
        (Rect::new(left, 0, area.width.saturating_sub(left), area.height), area)
    } else {
        let avail = area.width.saturating_sub(left);
        let w = state.panel_sidebar_w.min(avail / 2);
        (Rect::new(area.width.saturating_sub(w), 0, w, area.height), area)
    };

    if !panel_area.is_empty()
        && col >= panel_area.x
        && col < panel_area.right()
        && row >= panel_area.y
        && row < panel_area.bottom()
    {
        state.panel_focused = true;
        state.dirty = true;

        if let Some(panel) = state
            .mods_data
            .mods
            .iter()
            .flat_map(|m| &m.data.panels)
            .nth(state.active_right_sidebar)
            .cloned()
        {
            if let Some(fi) = state.panel_view.action_at(col, row) {
                state.panel_state.active_action = fi;
                if let Some(hint) = panel.footer.get(fi) {
                    let tab_id = panel
                        .tabs
                        .get(state.panel_state.active_tab)
                        .map(|t| t.id.as_str())
                        .unwrap_or("status");
                    let active_rows = panel.tab_rows.get(tab_id).unwrap_or(&panel.rows);
                    let selected_id = active_rows
                        .get(state.panel_state.selected)
                        .map(|r| r.id.clone())
                        .unwrap_or_default();
                    let action_name = match hint.key.as_str() {
                        "a" => "stage",
                        "d" => "diff_file",
                        "u" => "unstage",
                        "c" => "commit",
                        k => k,
                    };
                    let value = if action_name == "commit" {
                        serde_json::json!({ "action": "commit", "args": [] }).to_string()
                    } else if !selected_id.is_empty() {
                        serde_json::json!({ "action": action_name, "args": [selected_id] }).to_string()
                    } else {
                        panel.default_value.clone()
                    };

                    if !value.is_empty() {
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
                    }
                }
                return true;
            }
        }

        if let Some(is_next) = state.panel_view.arrow_at(col, row) {
            let panel = state
                .mods_data
                .mods
                .iter()
                .find_map(|m| m.data.panels.first().cloned());
            if let Some(panel) = panel {
                let n = panel.tabs.len();
                if n > 0 {
                    let cur = state.panel_state.active_tab;
                    let next = if is_next {
                        (cur + 1) % n
                    } else {
                        (cur + n - 1) % n
                    };
                    state.panel_state.set_active_tab(next, &panel);
                    state.dirty = true;
                }
            }
            return true;
        }

        if let Some(tab_idx) = state
            .panel_view
            .tab_at(col, row)
            .or_else(|| state.panel_view.tab_at_y(row))
        {
            let panel = state
                .mods_data
                .mods
                .iter()
                .find_map(|m| m.data.panels.first().cloned());
            if let Some(panel) = panel {
                state.panel_state.set_active_tab(tab_idx, &panel);
                state.dirty = true;
            }
            return true;
        }

        if let Some(row_idx) = state.panel_view.row_at_y(row) {
            let panel = state
                .mods_data
                .mods
                .iter()
                .flat_map(|m| &m.data.panels)
                .nth(state.active_right_sidebar)
                .cloned();
            if let Some(panel) = panel {
                state.panel_state.select(row_idx, &panel);
                state.panel_focused = true;
                state.dirty = true;
            }
            return true;
        }
        return true;
    } else if state.panel_sidebar_open || state.panel_maximized {
        state.panel_focused = false;
        state.dirty = true;
    }
    false
}

pub fn handle_active_modal_click(
    state: &mut AppState,
    col: u16,
    row: u16,
    area: Rect,
) {
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
                    crate::mux_core::modals::sync_modal_toggles(state);
                    return;
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
            crate::mux_core::modals::sync_modal_toggles(state);
            return;
        }
    }
    crate::mux_core::modals::sync_modal_toggles(state);
    state.active_modal = None;
}

