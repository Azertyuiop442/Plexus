
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

    let (panel_area, _) = if state.sidebar_open && area.width > state.sidebar_w + 8 {
        let left = state.sidebar_w;
        if state.panel_maximized {
            (Rect::new(left, 0, area.width - left, area.height), area)
        } else {
            let w = state.panel_sidebar_w.min(area.width.saturating_sub(left));
            (Rect::new(area.width - w, 0, w, area.height), area)
        }
    } else if state.panel_maximized {
        (area, area)
    } else {
        (Rect::default(), area)
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
            .find_map(|m| m.data.panels.first().cloned())
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
                .find_map(|m| m.data.panels.first().cloned());
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

